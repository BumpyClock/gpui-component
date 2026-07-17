//! The standard command set: App, Edit, and Window blocks, projected from the
//! registry into the native menu bar.
//!
//! App-scoped commands whose behavior the shell owns (Quit → the single
//! [`request_quit`](crate::AppShellExt::request_quit) path; macOS Hide/Show)
//! get a global `on_action` handler here. Commands whose behavior belongs to the
//! app or a window are registered as vocabulary only:
//!
//! - [`About`] — the app supplies its own handler (about window/panel).
//! - [`CloseWindow`], [`Minimize`], [`Zoom`] — window-scoped; the window manager
//!   wires these (flagged as an integration need).
//! - The Edit block — [`gpui_component::input`] actions the focused input already
//!   handles; no shell handler and no rebinding (the input component owns the
//!   `Input`-context keybindings).

use gpui::{App, OsAction, actions};
use gpui_component::input;

use super::{APP_MENU, AppCommandsExt, Command, CommandId, CommandScope, EDIT_MENU, WINDOW_MENU};
use crate::handles::AppShellExt;

actions!(
    app,
    [
        /// Quit the application through the shell's single shutdown path.
        Quit,
        /// Show the application's about window (app-provided handler).
        About,
        /// Close the focused window (window-scoped).
        CloseWindow,
    ]
);

#[cfg(target_os = "macos")]
actions!(
    app,
    [
        /// Hide the application (macOS).
        HideApp,
        /// Hide other applications (macOS).
        HideOthers,
        /// Show all applications (macOS).
        ShowAll,
        /// Minimize the focused window (macOS; window-scoped).
        Minimize,
        /// Zoom the focused window (macOS; window-scoped).
        Zoom,
    ]
);

/// Register the standard commands and the app-scoped handlers the shell owns.
pub(super) fn install(cx: &mut App) {
    register_app_block(cx);
    register_edit_block(cx);
    register_window_block(cx);
    register_handlers(cx);
}

/// App menu: About, (macOS Hide group), Quit.
fn register_app_block(cx: &mut App) {
    cx.register_command(
        Command::new(CommandId("app.about"), "About", CommandScope::App, About)
            .placed(APP_MENU, 0, 0),
    );

    #[cfg(target_os = "macos")]
    {
        cx.register_command(
            Command::new(CommandId("app.hide"), "Hide", CommandScope::App, HideApp)
                .with_binding("cmd-h")
                .placed(APP_MENU, 1, 0),
        );
        cx.register_command(
            Command::new(
                CommandId("app.hide_others"),
                "Hide Others",
                CommandScope::App,
                HideOthers,
            )
            .placed(APP_MENU, 1, 1),
        );
        cx.register_command(
            Command::new(
                CommandId("app.show_all"),
                "Show All",
                CommandScope::App,
                ShowAll,
            )
            .placed(APP_MENU, 1, 2),
        );
    }

    // Quit sits in a high group so it is always last, separated from the rest.
    let quit =
        Command::new(CommandId("app.quit"), "Quit", CommandScope::App, Quit).placed(APP_MENU, 9, 0);
    #[cfg(target_os = "macos")]
    let quit = quit.with_binding("cmd-q");
    cx.register_command(quit);
}

/// Edit menu: Undo/Redo, Cut/Copy/Paste, Select All (dispatched to the focused
/// input; no shell handler, no rebinding).
fn register_edit_block(cx: &mut App) {
    cx.register_command(edit(
        CommandId("edit.undo"),
        "Undo",
        input::Undo,
        OsAction::Undo,
        0,
        0,
    ));
    cx.register_command(edit(
        CommandId("edit.redo"),
        "Redo",
        input::Redo,
        OsAction::Redo,
        0,
        1,
    ));
    cx.register_command(edit(
        CommandId("edit.cut"),
        "Cut",
        input::Cut,
        OsAction::Cut,
        1,
        0,
    ));
    cx.register_command(edit(
        CommandId("edit.copy"),
        "Copy",
        input::Copy,
        OsAction::Copy,
        1,
        1,
    ));
    cx.register_command(edit(
        CommandId("edit.paste"),
        "Paste",
        input::Paste,
        OsAction::Paste,
        1,
        2,
    ));
    cx.register_command(edit(
        CommandId("edit.select_all"),
        "Select All",
        input::SelectAll,
        OsAction::SelectAll,
        2,
        0,
    ));
}

/// Build one Edit command (window-scoped, with its OS action and placement).
fn edit(
    id: CommandId,
    label: &'static str,
    action: impl gpui::Action,
    os_action: OsAction,
    group: u16,
    order: u16,
) -> Command {
    Command::new(id, label, CommandScope::Window, action)
        .with_os_action(os_action)
        .placed(EDIT_MENU, group, order)
}

/// Window menu: (macOS Minimize/Zoom), Close Window.
fn register_window_block(cx: &mut App) {
    #[cfg(target_os = "macos")]
    {
        cx.register_command(
            Command::new(
                CommandId("window.minimize"),
                "Minimize",
                CommandScope::Window,
                Minimize,
            )
            .with_binding("cmd-m")
            .placed(WINDOW_MENU, 0, 0),
        );
        cx.register_command(
            Command::new(CommandId("window.zoom"), "Zoom", CommandScope::Window, Zoom).placed(
                WINDOW_MENU,
                0,
                1,
            ),
        );
    }

    let close = Command::new(
        CommandId("window.close"),
        "Close Window",
        CommandScope::Window,
        CloseWindow,
    )
    .placed(WINDOW_MENU, 1, 0);
    #[cfg(target_os = "macos")]
    let close = close.with_binding("cmd-w");
    cx.register_command(close);
}

/// Register the global handlers the shell owns. Quit routes through the single
/// shutdown path; the macOS Hide/Show actions call the platform directly.
fn register_handlers(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.request_quit());

    #[cfg(target_os = "macos")]
    {
        cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
        cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
        cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    }
}
