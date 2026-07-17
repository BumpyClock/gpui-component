//! [`MenusPlugin`]: installs the standard command set, registers default
//! keybindings, and keeps the native menu bar in sync with the registry.
//!
//! Reactive rebuild: the plugin observes the [`gpui_component::Theme`] global so
//! checked state (appearance mode, active theme) refreshes automatically, and
//! exposes [`menus_invalidate`] for other modules (theme registry watcher, window
//! manager) to call when their contributing sections change.
//!
//! Keybinding precedence (component < shell < app < user): the component tier is
//! bound by `gpui_component::init` before this plugin runs; default bindings are
//! then registered from the registry in registration order, and GPUI resolves
//! later-registered bindings first — so shell/app defaults override component
//! defaults, and a future user keymap (registered last) will override both.

use gpui::{Action, App, DummyKeyboardMapper, KeyBinding, KeyBindingMetaIndex, Subscription};

use super::{CommandId, CommandRegistry, MenuPlan, menu, standard};
use crate::error::AppShellError;
use crate::plugin::{AppPlugin, ShellSeed, sealed};

/// Meta tag stamped on every keybinding this module installs, so a rebuild can
/// tell registry-owned bindings apart from the component tier (bound by
/// `gpui_component::init`) and any future user-keymap tier — neither of which we
/// own — and drop only ours before re-adding the current registry.
const COMMANDS_BINDING_META: KeyBindingMetaIndex = KeyBindingMetaIndex(0xC0FFEE);

/// Installs and maintains the native menu bar from the command registry.
pub struct MenusPlugin {
    plan: Option<MenuPlan>,
    theme_observer: Option<Subscription>,
}

impl MenusPlugin {
    /// A plugin driving the given [`MenuPlan`].
    pub fn new(plan: MenuPlan) -> Self {
        Self {
            plan: Some(plan),
            theme_observer: None,
        }
    }

    /// A plugin driving the standard App + Edit + Window menus.
    pub fn standard() -> Self {
        Self::new(MenuPlan::standard())
    }
}

impl sealed::Sealed for MenusPlugin {}

impl AppPlugin for MenusPlugin {
    fn init(&mut self, cx: &mut App, _shell: &ShellSeed) -> Result<(), AppShellError> {
        // Register the standard commands and the shell-owned handlers; this also
        // creates the registry global if a contributor has not already.
        standard::install(cx);

        // Shell/app-tier default bindings, in registration order (component tier
        // was bound by gpui_component::init already).
        install_bindings(cx);

        let plan = self.plan.take().unwrap_or_else(MenuPlan::standard);
        cx.global_mut::<CommandRegistry>().set_plan(plan);

        // Initial projection, then rebuild whenever the theme changes.
        menus_invalidate(cx);

        // Mark the registry live: past this point apps can only register commands
        // from `on_launch` (or later), so those registrations must bind and
        // re-project themselves — see `AppCommandsExt::register_command`.
        cx.global_mut::<CommandRegistry>().activate();

        self.theme_observer =
            Some(cx.observe_global::<gpui_component::Theme>(|cx| menus_invalidate(cx)));
        Ok(())
    }
}

/// Bind a *newly added* command's default keybinding (if any) and re-project the
/// menus.
///
/// Append-only fast path for a brand-new command id registered after the initial
/// pass (the common `on_launch` case). The binding lands after the shell defaults
/// and below any future user overrides — preserving precedence. For *replacing*
/// an existing id, use [`rebuild_bindings_and_reproject`], which also removes the
/// replaced command's now-stale chord.
pub(super) fn bind_and_reproject(cx: &mut App, id: CommandId) {
    if let Some(binding) = binding_for(cx, id) {
        cx.bind_keys([binding]);
    }
    menus_invalidate(cx);
}

/// Rebuild the registry's keybindings from scratch and re-project the menus.
///
/// Used when an already-registered id is replaced after the initial pass: the
/// old command's binding is still live in the keymap, and the fork exposes no
/// per-binding removal — only `clear_key_bindings` (all) plus `bind_keys`. So we
/// snapshot the whole keymap, drop the bindings we own (tagged), and restore the
/// foreign ones (component tier, any user tier) *in place* around a freshly
/// rebuilt registry: those installed before our tier stay before it, those
/// installed after stay after (see [`split_around_registry`]). Net effect: stale
/// registry chords disappear while every foreign tier keeps its bindings, order,
/// and precedence relative to ours.
pub(super) fn rebuild_bindings_and_reproject(cx: &mut App) {
    let snapshot: Vec<KeyBinding> = {
        let keymap = cx.key_bindings();
        let keymap = keymap.borrow();
        keymap.bindings().cloned().collect()
    };
    let (before, after) = split_around_registry(snapshot);
    cx.clear_key_bindings();
    cx.bind_keys(before);
    install_bindings(cx);
    cx.bind_keys(after);
    menus_invalidate(cx);
}

/// Split a keymap snapshot around the registry tier, dropping the registry-owned
/// (tagged) bindings so they can be rebuilt fresh.
///
/// Foreign bindings that appeared *before* the first registry-owned binding are
/// returned in `before`; every other foreign binding — including any interleaved
/// with or following the registry tier — is returned in `after`. Rebuilding as
/// `before + registry + after` preserves each foreign binding's position, and
/// thus precedence, relative to our tier.
fn split_around_registry(snapshot: Vec<KeyBinding>) -> (Vec<KeyBinding>, Vec<KeyBinding>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut seen_registry = false;
    for binding in snapshot {
        if binding.meta() == Some(COMMANDS_BINDING_META) {
            seen_registry = true;
            continue;
        }
        if seen_registry {
            after.push(binding);
        } else {
            before.push(binding);
        }
    }
    (before, after)
}

/// Build the [`KeyBinding`] for a registered command's default binding, if it has
/// one.
fn binding_for(cx: &App, id: CommandId) -> Option<KeyBinding> {
    let registry = cx.global::<CommandRegistry>();
    let command = registry.get(id)?;
    let keystroke = command.default_binding()?;
    Some(load_binding(keystroke, command.boxed_action()))
}

/// Construct a global (context-less) keybinding from a keystroke and boxed
/// action, tagged as registry-owned so a rebuild can identify and replace it.
fn load_binding(keystroke: &str, action: Box<dyn Action>) -> KeyBinding {
    KeyBinding::load(keystroke, action, None, false, None, &DummyKeyboardMapper)
        .expect("command default binding must parse")
        .with_meta(COMMANDS_BINDING_META)
}

/// Re-project the registry into the native menu bar (and the macOS dock menu).
///
/// Call this from any module whose contributing section or command state changed
/// (theme registry updates, window list changes, enabled/checked transitions).
/// A no-op before the registry exists.
pub fn menus_invalidate(cx: &mut App) {
    if !cx.has_global::<CommandRegistry>() {
        return;
    }
    let menus = {
        let registry = cx.global::<CommandRegistry>();
        menu::build_menus(cx, registry)
    };
    if let Some(menus) = menus {
        cx.set_menus(menus);
    }

    #[cfg(target_os = "macos")]
    {
        // Set unconditionally: an empty projection must clear a previously
        // installed dock menu (e.g. the last dock-placed command was moved off
        // the dock), otherwise the stale native item stays live.
        let items = {
            let registry = cx.global::<CommandRegistry>();
            menu::build_dock_items(cx, registry)
        };
        cx.set_dock_menu(items);
    }
}

/// Register default keybindings from the registry, in registration order. GPUI
/// resolves bindings in reverse insertion order, so later registrations win.
fn install_bindings(cx: &mut App) {
    let bindings: Vec<KeyBinding> = {
        let registry = cx.global::<CommandRegistry>();
        registry
            .commands()
            .iter()
            .filter_map(|command| {
                command
                    .default_binding()
                    .map(|keystroke| load_binding(keystroke, command.boxed_action()))
            })
            .collect()
    };
    cx.bind_keys(bindings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{
        APP_MENU, AppCommandsExt, Command, CommandId, CommandRegistry, CommandScope, EDIT_MENU,
    };
    use gpui::actions;

    actions!(plugin_test, [A, B, C]);

    /// The pure ordered list of (id, keystroke) that `install_bindings` feeds to
    /// `bind_keys`, mirroring registration order (later = higher precedence).
    fn binding_order(registry: &CommandRegistry) -> Vec<(CommandId, &'static str)> {
        registry
            .commands()
            .iter()
            .filter_map(|c| c.default_binding().map(|k| (c.id(), k)))
            .collect()
    }

    #[test]
    fn split_around_registry_preserves_foreign_positions() {
        // component tier (before) | two registry (tagged) | app raw bind (after).
        let snapshot = vec![
            KeyBinding::new("ctrl-a", A, None),
            load_binding("cmd-1", Box::new(B)),
            load_binding("cmd-2", Box::new(B)),
            KeyBinding::new("ctrl-b", C, None),
        ];
        let (before, after) = split_around_registry(snapshot);
        let names = |v: &[KeyBinding]| v.iter().map(|b| b.action().name()).collect::<Vec<_>>();
        // Registry-owned bindings dropped; foreign bindings keep their side.
        assert_eq!(names(&before), vec![A.name()]);
        assert_eq!(names(&after), vec![C.name()]);
    }

    #[test]
    fn bindings_follow_registration_order() {
        let mut registry = CommandRegistry::new();
        registry.register(
            Command::new(CommandId("a"), "A", CommandScope::App, A)
                .with_binding("cmd-k")
                .with_placement(super::super::MenuPlacement::new(APP_MENU, 0, 0)),
        );
        // No binding -> excluded.
        registry.register(Command::new(CommandId("b"), "B", CommandScope::App, B));
        registry.register(
            Command::new(CommandId("c"), "C", CommandScope::App, C)
                .with_binding("cmd-k")
                .with_placement(super::super::MenuPlacement::new(EDIT_MENU, 0, 0)),
        );

        // "c" comes after "a": it is registered later, so it wins the shared
        // "cmd-k" chord under GPUI's reverse-order resolution.
        assert_eq!(
            binding_order(&registry),
            vec![(CommandId("a"), "cmd-k"), (CommandId("c"), "cmd-k")]
        );
    }

    #[test]
    fn post_activation_registration_exposes_binding_and_placement() {
        use crate::commands::{MenuNode, MenuPlacement};

        let mut registry = CommandRegistry::new();
        // Initial batch, bound by `install_bindings`.
        registry.register(
            Command::new(CommandId("std"), "Std", CommandScope::App, A).with_binding("cmd-1"),
        );
        assert!(!registry.is_active());

        // The plugin has finished its initial pass.
        registry.activate();
        assert!(registry.is_active());

        // An app registers a new command from `on_launch` (post-activation). The
        // ext path would call `bind_and_reproject`; the registry state it reads is
        // what we assert here.
        registry.register(
            Command::new(CommandId("late"), "Late", CommandScope::App, B)
                .with_binding("cmd-2")
                .with_placement(MenuPlacement::new(APP_MENU, 0, 0)),
        );

        // The late binding is present, ordered after the initial one (so it wins
        // a shared chord and sits in the app tier).
        assert_eq!(
            binding_order(&registry),
            vec![(CommandId("std"), "cmd-1"), (CommandId("late"), "cmd-2")]
        );
        // And the late command's placement now appears in the projected outline.
        let outline = MenuPlan::standard().outline(registry.commands());
        assert!(
            outline[0]
                .nodes
                .contains(&MenuNode::Command(CommandId("late")))
        );
    }

    #[test]
    fn app_command_ext_creates_registry_lazily() {
        // The ext trait's lazy-create path is covered structurally: with no
        // global, command_registry is None; the mutation path is exercised by the
        // headless integration test.
        fn assert_impls<T: AppCommandsExt>() {}
        assert_impls::<App>();
    }
}
