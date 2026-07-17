//! The `AppShell` builder and phase-driven `run` (plan §3).
//!
//! Builder methods only *record intent*; [`AppShellBuilder::run`] executes the
//! fixed phase order from [`crate::phases`]. Work that must happen before the
//! GPUI event loop (path resolution, env/logging policy, `before_platform`,
//! plugin `configure`) runs in `Preflight`; everything else runs inside the run
//! closure with a raw `&mut gpui::App`.

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gpui::{App, Application, AssetSource, QuitMode, SharedString};
use gpui_component_manifest::schema::IdentityRef;
use gpui_component_storage::{AppPaths, PathLayout};

use crate::capabilities::PlatformCapabilities;
use crate::error::AppShellError;
use crate::handles::{self, AppInfo, PendingEvents};
use crate::lifecycle::{AppEvent, LaunchRequest};
use crate::liveness::{ExitPolicy, InitialActivation, Liveness};
use crate::phases::{Phase, PhaseTracker};
use crate::plugin::{AppPlugin, BuildContext, EventHandler, ShellSeed};

/// Process-global environment policy (plan §3 — explicit, never a silent
/// default). Applied once in `Preflight`.
#[non_exhaustive]
pub enum EnvironmentPolicy {
    /// Inherit the launching environment unchanged (default; safe for a library).
    Inherit,
    /// Repair `PATH` from the user's login shell.
    ///
    /// Not implemented in core: `fix-path-env` is not a dependency. Selecting
    /// this logs a warning and behaves as [`EnvironmentPolicy::Inherit`]; apps
    /// that need it should run their own repair in `before_platform`.
    LoginShell,
}

// There is deliberately no `Custom(vars)` variant: `std::env::set_var` is
// unsafe (UB with concurrent environment access on Unix), and this builder
// cannot know whether the caller already spawned threads. Apps that need to
// mutate the environment must do so in their own `main()` before constructing
// the shell, where the safety obligation is visibly theirs.

/// Process-global logging policy (plan §3). The library must not seize the
/// process logger by default.
#[non_exhaustive]
pub enum LoggingPolicy {
    /// The application (or its harness) owns logging. Default.
    External,
    /// Run an app-provided initializer with resolved paths during `Preflight`.
    Configure(Box<dyn FnOnce(&AppPaths)>),
}

/// Selects the GPUI platform backend. Injected for testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformRunner {
    kind: RunnerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerKind {
    Native,
    Headless,
}

impl PlatformRunner {
    /// The real platform for the current OS.
    pub fn native() -> Self {
        Self {
            kind: RunnerKind::Native,
        }
    }

    /// A headless platform, for bootstrap/lifecycle tests.
    pub fn headless() -> Self {
        Self {
            kind: RunnerKind::Headless,
        }
    }

    fn build(self) -> Application {
        match self.kind {
            RunnerKind::Native => gpui_platform::application(),
            RunnerKind::Headless => gpui_platform::headless(),
        }
    }
}

impl Default for PlatformRunner {
    fn default() -> Self {
        Self::native()
    }
}

/// Entry point: `AppShell::builder(APP_IDENTITY)`.
pub struct AppShell;

impl AppShell {
    /// Start building an application shell from the compiled-in identity.
    ///
    /// `identity` is the `APP_IDENTITY` produced by `include_identity!()`.
    pub fn builder(identity: IdentityRef) -> AppShellBuilder {
        AppShellBuilder::new(identity)
    }
}

/// Accumulates configuration; [`AppShellBuilder::run`] executes it.
pub struct AppShellBuilder {
    identity: IdentityRef,
    assets: Vec<Arc<dyn AssetSource>>,
    path_layout: PathLayout,
    environment: EnvironmentPolicy,
    logging: LoggingPolicy,
    initial_activation: InitialActivation,
    exit_policy: ExitPolicy,
    configure_app: Option<Box<dyn FnOnce(Application) -> Result<Application, AppShellError>>>,
    before_platform: Vec<Box<dyn FnOnce() -> Result<(), AppShellError>>>,
    state: HashMap<TypeId, Box<dyn Any>>,
    plugins: Vec<Box<dyn AppPlugin>>,
    handlers: Vec<EventHandler>,
    runner: PlatformRunner,
}

impl AppShellBuilder {
    fn new(identity: IdentityRef) -> Self {
        Self {
            identity,
            assets: Vec::new(),
            path_layout: PathLayout::PlatformDefault,
            environment: EnvironmentPolicy::Inherit,
            logging: LoggingPolicy::External,
            initial_activation: InitialActivation::Regular,
            exit_policy: ExitPolicy::WhenIdle,
            configure_app: None,
            before_platform: Vec::new(),
            state: HashMap::new(),
            // Always-present services, installed first so user plugins can rely
            // on them during their own init: the platform-owned preferences
            // store (theme mode/name, locale) and the window manager.
            plugins: vec![
                Box::new(crate::settings::ShellPreferencesPlugin::new()),
                Box::new(crate::windows::WindowsPlugin::new()),
            ],
            handlers: Vec::new(),
            runner: PlatformRunner::native(),
        }
    }

    /// Append an asset source. Sources are tried in registration order; the
    /// first to resolve a path wins (no silent last-writer-wins).
    pub fn assets(mut self, source: impl AssetSource) -> Self {
        self.assets.push(Arc::new(source));
        self
    }

    /// Choose the on-disk directory layout (default [`PathLayout::PlatformDefault`]).
    pub fn path_layout(mut self, layout: PathLayout) -> Self {
        self.path_layout = layout;
        self
    }

    /// Set the environment policy (default [`EnvironmentPolicy::Inherit`]).
    pub fn environment(mut self, policy: EnvironmentPolicy) -> Self {
        self.environment = policy;
        self
    }

    /// Set the logging policy (default [`LoggingPolicy::External`]).
    pub fn logging(mut self, policy: LoggingPolicy) -> Self {
        self.logging = policy;
        self
    }

    /// Set the initial-activation policy. Use [`InitialActivation::Passive`] for
    /// tray-first apps (default [`InitialActivation::Regular`]).
    pub fn initial_activation(mut self, activation: InitialActivation) -> Self {
        self.initial_activation = activation;
        self
    }

    /// Set the exit policy. Use [`ExitPolicy::Explicit`] for apps that outlive
    /// their windows (default [`ExitPolicy::WhenIdle`]).
    pub fn exit_policy(mut self, policy: ExitPolicy) -> Self {
        self.exit_policy = policy;
        self
    }

    /// Customize the GPUI [`Application`] before it runs (e.g. HTTP client).
    pub fn configure_application(
        mut self,
        f: impl FnOnce(Application) -> Result<Application, AppShellError> + 'static,
    ) -> Self {
        self.configure_app = Some(Box::new(f));
        self
    }

    /// Register a stateless side effect to run before the platform starts. Runs
    /// on the main thread with no GPUI and no windows.
    pub fn before_platform(
        mut self,
        f: impl FnOnce() -> Result<(), AppShellError> + 'static,
    ) -> Self {
        self.before_platform.push(Box::new(f));
        self
    }

    /// Register typed state prepared before the platform exists (e.g. audio
    /// bootstrap). Retrieve it later via `cx.app_state::<T>()`.
    pub fn state<T: 'static>(mut self, value: T) -> Self {
        self.state.insert(TypeId::of::<T>(), Box::new(value));
        self
    }

    /// Install an internal plugin. Order is preserved for init; shutdown runs in
    /// reverse.
    pub fn plugin(mut self, plugin: impl AppPlugin) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Handle every lifecycle [`AppEvent`].
    pub fn on_event(
        mut self,
        mut handler: impl FnMut(&AppEvent, &mut App) -> Result<(), AppShellError> + 'static,
    ) -> Self {
        self.handlers
            .push(Box::new(move |event, cx| handler(event, cx)));
        self
    }

    /// Sugar over `on_event(Started)`: run `f` once the app has launched.
    pub fn on_launch(
        self,
        mut f: impl FnMut(&mut App) -> Result<(), AppShellError> + 'static,
    ) -> Self {
        self.on_event(move |event, cx| {
            if matches!(event, AppEvent::Started(_)) {
                f(cx)
            } else {
                Ok(())
            }
        })
    }

    /// Sugar over `on_event(Reopened)`.
    pub fn on_reopen(
        self,
        mut f: impl FnMut(&mut App) -> Result<(), AppShellError> + 'static,
    ) -> Self {
        self.on_event(move |event, cx| {
            if matches!(event, AppEvent::Reopened) {
                f(cx)
            } else {
                Ok(())
            }
        })
    }

    /// Inject the platform runner (default native). Use
    /// [`PlatformRunner::headless`] in tests.
    pub fn runner(mut self, runner: PlatformRunner) -> Self {
        self.runner = runner;
        self
    }

    /// Execute the shell: run the fixed phase sequence, then the GPUI event loop.
    pub fn run(self) -> Result<(), AppShellError> {
        let Self {
            identity,
            assets,
            path_layout,
            environment,
            logging,
            initial_activation,
            exit_policy,
            configure_app,
            before_platform,
            state,
            mut plugins,
            handlers,
            runner,
        } = self;

        // ---- Preflight (no GPUI) ----
        validate_identity(&identity)?;
        let paths = AppPaths::new(identity.data_namespace, path_layout)?;
        apply_environment(environment);
        apply_logging(logging, &paths);
        for hook in before_platform {
            hook()?;
        }
        let capabilities = PlatformCapabilities::detect();
        let app_info = AppInfo::new(identity, paths, capabilities);

        // Plugin configure() sees identity/paths/capabilities before GPUI exists.
        for plugin in &mut plugins {
            let mut ctx = BuildContext { info: &app_info };
            plugin.configure(&mut ctx);
        }

        let launch = LaunchRequest::from_env();

        // ---- ConfigureApp ----
        let mut application = runner.build().with_assets(ChainedAssets::new(assets));
        if let Some(configure) = configure_app {
            application = configure(application)?;
        }
        // Quit mode is shell-owned and not customizable: liveness is the single
        // quit authority, so every quit routes through `request_quit`. Applied
        // AFTER `configure_application` so the app callback cannot re-enable
        // platform auto-quit and bypass the lifecycle/teardown path.
        application = application.with_quit_mode(QuitMode::Explicit);

        // ---- EarlyListeners ----
        let pending = Arc::new(PendingEvents::default());
        {
            let pending = Arc::clone(&pending);
            application.on_open_urls(move |urls| {
                pending.push(handles::open_event(urls));
            });
        }
        {
            // Route early reopens through the same queue-until-ready buffer as
            // URLs; delivering directly here would be a no-op before the shell
            // global is installed, silently losing a startup-time reopen.
            let pending = Arc::clone(&pending);
            application.on_reopen(move |_cx| {
                pending.push(AppEvent::Reopened);
            });
        }

        // ---- run: the remaining phases execute on the main thread ----
        let error_cell: Arc<Mutex<Option<AppShellError>>> = Arc::new(Mutex::new(None));
        let liveness = Liveness::new(exit_policy, initial_activation);
        let boot = Boot {
            app_info,
            liveness,
            initial_activation,
            plugins,
            handlers,
            pending,
            state,
            launch,
        };
        let error_slot = Arc::clone(&error_cell);
        application.run(move |cx| {
            if let Err(err) = boot.run(cx) {
                // On macOS `cx.quit()` terminates the process with status 0
                // before `Application::run` returns, which would convert a
                // startup failure into a successful exit (and let broken boots
                // pass the CI smoke gate). Initialized plugins were already
                // unwound in reverse by `boot.run`'s error path, so exit
                // directly with a failing status. On platforms where `run`
                // returns, the error cell below still reports the same error.
                log::error!("app shell startup failed: {err}");
                *error_slot.lock().expect("error cell poisoned") = Some(err);
                if cfg!(target_os = "macos") {
                    std::process::exit(2);
                }
                cx.quit();
            }
        });

        match error_cell.lock().expect("error cell poisoned").take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

/// Everything moved into the run closure to execute the post-`ConfigureApp`
/// phases on the main thread.
struct Boot {
    app_info: AppInfo,
    liveness: Liveness,
    initial_activation: InitialActivation,
    plugins: Vec<Box<dyn AppPlugin>>,
    handlers: Vec<EventHandler>,
    pending: Arc<PendingEvents>,
    state: HashMap<TypeId, Box<dyn Any>>,
    launch: LaunchRequest,
}

impl Boot {
    fn run(self, cx: &mut App) -> Result<(), AppShellError> {
        let Self {
            app_info,
            liveness,
            initial_activation,
            mut plugins,
            handlers,
            pending,
            state,
            launch,
        } = self;

        let mut phases = PhaseTracker::new();
        phases.complete(Phase::Preflight);
        phases.complete(Phase::ConfigureApp);
        phases.complete(Phase::EarlyListeners);

        // ComponentInit
        gpui_component::init(cx);
        phases.complete(Phase::ComponentInit);

        // CoreServices: install the global (moves plugins/handlers/state in) and
        // start the cross-thread drain loop.
        let proxy = handles::install(
            cx,
            app_info.clone(),
            liveness,
            std::mem::take(&mut plugins),
            handlers,
            Arc::clone(&pending),
            state,
            phases,
        );
        cx.global_mut::<crate::handles::ShellState>()
            .record_phase(Phase::CoreServices);

        // Install lifecycle observers (incl. the `on_app_quit` teardown hook)
        // immediately, BEFORE any application-controlled handler (plugin init,
        // `Started`/`on_launch`) can run. Otherwise a `request_quit()` from an
        // `on_launch` handler would terminate before the quit observer exists,
        // skipping WillExit, reverse plugin shutdown, proxy close, and flush.
        handles::register_observers(cx);

        // PluginInit: initialize plugins with the shell seed. A failure here is
        // fatal (required service); it aborts startup, unwinding the already-
        // initialized prefix in reverse (the documented shutdown contract).
        let seed = ShellSeed {
            info: app_info,
            proxy: proxy.clone(),
        };
        let mut installed = cx.global_mut::<crate::handles::ShellState>().take_plugins();
        let mut initialized = 0usize;
        let mut init_error = None;
        for plugin in &mut installed {
            match plugin.init(cx, &seed) {
                Ok(()) => initialized += 1,
                Err(err) => {
                    init_error = Some(err);
                    break;
                }
            }
        }
        if let Some(err) = init_error {
            // The failing plugin never completed init and is not shut down; the
            // successfully-initialized prefix is torn down in reverse order.
            for plugin in installed[..initialized].iter_mut().rev() {
                plugin.shutdown(cx);
            }
            return Err(err);
        }
        cx.global_mut::<crate::handles::ShellState>()
            .restore_plugins(installed);
        cx.global_mut::<crate::handles::ShellState>()
            .record_phase(Phase::PluginInit);

        // DrainQueue: deliver Started, enable post-ready delivery, drain buffer.
        handles::deliver_event(cx, &AppEvent::Started(launch));
        let _ = pending.proxy.set(proxy);
        handles::drain_pending(cx);
        cx.global_mut::<crate::handles::ShellState>()
            .record_phase(Phase::DrainQueue);

        // Activation.
        match initial_activation {
            InitialActivation::Regular => cx.activate(false),
            InitialActivation::Passive => {}
        }
        cx.global_mut::<crate::handles::ShellState>()
            .record_phase(Phase::Activation);

        // Startup has reached its stable state. Evaluate exit once so an app
        // that launched genuinely idle (no windows opened during `Started`, no
        // holds) exits under `ExitPolicy::WhenIdle` instead of living forever
        // under the shell-owned `QuitMode::Explicit`. Otherwise nothing would
        // ever trigger evaluation (which only fires on window-close/hold-drop).
        // No window closed here, so no pending reason is set → attributed to
        // `Requested`.
        handles::evaluate_exit(cx);
        Ok(())
    }
}

fn validate_identity(identity: &IdentityRef) -> Result<(), AppShellError> {
    if identity.app_id.is_empty() {
        return Err(AppShellError::Identity("app_id is empty".into()));
    }
    if identity.data_namespace.is_empty() {
        return Err(AppShellError::Identity("data_namespace is empty".into()));
    }
    Ok(())
}

fn apply_environment(policy: EnvironmentPolicy) {
    match policy {
        EnvironmentPolicy::Inherit => {}
        EnvironmentPolicy::LoginShell => {
            log::warn!(
                "EnvironmentPolicy::LoginShell is not implemented in core; inheriting environment"
            );
        }
    }
}

fn apply_logging(policy: LoggingPolicy, paths: &AppPaths) {
    match policy {
        LoggingPolicy::External => {}
        LoggingPolicy::Configure(init) => init(paths),
    }
}

/// Composes asset sources with first-match-wins load and unioned listings.
struct ChainedAssets {
    sources: Vec<Arc<dyn AssetSource>>,
}

impl ChainedAssets {
    fn new(sources: Vec<Arc<dyn AssetSource>>) -> Self {
        Self { sources }
    }
}

impl AssetSource for ChainedAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        for source in &self.sources {
            if let Some(bytes) = source.load(path)? {
                return Ok(Some(bytes));
            }
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut out = Vec::new();
        for source in &self.sources {
            out.extend(source.list(path)?);
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}
