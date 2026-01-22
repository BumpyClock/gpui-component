use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, Styled, Task, Window, div, px,
};
use smol::Timer;
use std::sync::Arc;
use std::time::Duration;

use gpui_component::{
    ActiveTheme, Icon, IconName, WindowExt as _,
    button::{Button, ButtonVariants as _},
    command_palette::{
        CommandPalette, CommandPaletteConfig, CommandPaletteEvent, CommandPaletteItem,
        CommandPaletteProvider, StaticProvider,
    },
    h_flex, v_flex,
};

use crate::section;

pub struct CommandPaletteStory {
    focus_handle: FocusHandle,
    last_selected: Option<SharedString>,
}

impl super::Story for CommandPaletteStory {
    fn title() -> &'static str {
        "CommandPalette"
    }

    fn description() -> &'static str {
        "A modal command palette with fuzzy search and keyboard navigation"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl CommandPaletteStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            last_selected: None,
        }
    }

    fn show_static_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = vec![
            CommandPaletteItem::new("file.new", "New File")
                .category("File")
                .icon(IconName::FilePlus)
                .shortcut("cmd-n")
                .keyword("create"),
            CommandPaletteItem::new("file.open", "Open File")
                .category("File")
                .icon(IconName::FolderOpen)
                .shortcut("cmd-o")
                .keyword("browse"),
            CommandPaletteItem::new("file.save", "Save File")
                .category("File")
                .icon(IconName::Save)
                .shortcut("cmd-s"),
            CommandPaletteItem::new("file.save-all", "Save All")
                .category("File")
                .icon(IconName::Save)
                .shortcut("cmd-shift-s"),
            CommandPaletteItem::new("edit.undo", "Undo")
                .category("Edit")
                .icon(IconName::Undo)
                .shortcut("cmd-z"),
            CommandPaletteItem::new("edit.redo", "Redo")
                .category("Edit")
                .icon(IconName::Redo)
                .shortcut("cmd-shift-z"),
            CommandPaletteItem::new("edit.cut", "Cut")
                .category("Edit")
                .icon(IconName::Scissors)
                .shortcut("cmd-x"),
            CommandPaletteItem::new("edit.copy", "Copy")
                .category("Edit")
                .icon(IconName::Copy)
                .shortcut("cmd-c"),
            CommandPaletteItem::new("edit.paste", "Paste")
                .category("Edit")
                .icon(IconName::Clipboard)
                .shortcut("cmd-v"),
            CommandPaletteItem::new("search.find", "Find")
                .category("Search")
                .icon(IconName::Search)
                .shortcut("cmd-f")
                .keyword("locate"),
            CommandPaletteItem::new("search.replace", "Replace")
                .category("Search")
                .icon(IconName::Replace)
                .shortcut("cmd-r"),
            CommandPaletteItem::new("search.files", "Search Files")
                .category("Search")
                .icon(IconName::FileSearch)
                .shortcut("cmd-p"),
            CommandPaletteItem::new("view.terminal", "Toggle Terminal")
                .category("View")
                .icon(IconName::Terminal)
                .shortcut("cmd-`"),
            CommandPaletteItem::new("view.sidebar", "Toggle Sidebar")
                .category("View")
                .icon(IconName::PanelLeft)
                .shortcut("cmd-b"),
            CommandPaletteItem::new("git.commit", "Commit Changes")
                .category("Git")
                .icon(IconName::GitCommit)
                .shortcut("cmd-shift-c")
                .keyword("vcs"),
            CommandPaletteItem::new("git.push", "Push to Remote")
                .category("Git")
                .icon(IconName::Upload)
                .shortcut("cmd-shift-p"),
            CommandPaletteItem::new("git.pull", "Pull from Remote")
                .category("Git")
                .icon(IconName::Download)
                .keyword("fetch"),
            CommandPaletteItem::new("git.status", "Git Status")
                .category("Git")
                .icon(IconName::GitBranch),
        ];

        let provider = Arc::new(StaticProvider::new(items));
        let handle = CommandPalette::open(window, cx, provider);

        let view = cx.entity().clone();
        cx.subscribe(&handle.state(), move |_, event, window, cx| match event {
            CommandPaletteEvent::Selected { item } => {
                view.update(cx, |view, cx| {
                    view.last_selected = Some(item.title.clone());
                    cx.notify();
                });
                window.push_notification(
                    format!("Selected: {} ({})", item.title, item.id),
                    cx,
                );
            }
            CommandPaletteEvent::Dismissed => {
                window.push_notification("Command palette dismissed", cx);
            }
        })
        .detach();
    }

    fn show_async_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let provider = Arc::new(AsyncDemoProvider::new());
        let handle = CommandPalette::open(window, cx, provider);

        let view = cx.entity().clone();
        cx.subscribe(&handle.state(), move |_, event, window, cx| match event {
            CommandPaletteEvent::Selected { item } => {
                view.update(cx, |view, cx| {
                    view.last_selected = Some(item.title.clone());
                    cx.notify();
                });
                window.push_notification(
                    format!("Selected: {} ({})", item.title, item.id),
                    cx,
                );
            }
            CommandPaletteEvent::Dismissed => {
                window.push_notification("Command palette dismissed", cx);
            }
        })
        .detach();
    }

    fn show_custom_config_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = vec![
            CommandPaletteItem::new("action-1", "Action One")
                .category("Actions")
                .icon(IconName::Zap),
            CommandPaletteItem::new("action-2", "Action Two")
                .category("Actions")
                .icon(IconName::Zap),
            CommandPaletteItem::new("action-3", "Action Three")
                .category("Actions")
                .icon(IconName::Zap),
        ];

        let provider = Arc::new(StaticProvider::new(items));

        let custom_config = CommandPaletteConfig {
            placeholder: "Search actions...".into(),
            width: 700.0,
            max_height: 300.0,
            max_results: 10,
            show_footer: true,
            show_categories_inline: true,
            ..Default::default()
        };

        let handle = CommandPalette::open_with_config(window, cx, provider, custom_config);

        let view = cx.entity().clone();
        cx.subscribe(&handle.state(), move |_, event, window, cx| match event {
            CommandPaletteEvent::Selected { item } => {
                view.update(cx, |view, cx| {
                    view.last_selected = Some(item.title.clone());
                    cx.notify();
                });
                window.push_notification(format!("Selected: {}", item.title), cx);
            }
            CommandPaletteEvent::Dismissed => {}
        })
        .detach();
    }
}

/// Demo provider that combines static items with async search
struct AsyncDemoProvider {
    static_items: Vec<CommandPaletteItem>,
}

impl AsyncDemoProvider {
    fn new() -> Self {
        Self {
            static_items: vec![
                CommandPaletteItem::new("static.help", "Help")
                    .category("Static")
                    .icon(IconName::HelpCircle)
                    .shortcut("F1"),
                CommandPaletteItem::new("static.settings", "Settings")
                    .category("Static")
                    .icon(IconName::Settings)
                    .shortcut("cmd-,"),
                CommandPaletteItem::new("static.about", "About")
                    .category("Static")
                    .icon(IconName::Info),
            ],
        }
    }
}

impl CommandPaletteProvider for AsyncDemoProvider {
    fn items(&self, _cx: &App) -> Vec<CommandPaletteItem> {
        self.static_items.clone()
    }

    fn query(&self, query: &str, cx: &App) -> Task<Vec<CommandPaletteItem>> {
        if query.is_empty() {
            return Task::ready(Vec::new());
        }

        let query = query.to_lowercase();

        cx.background_spawn(async move {
            // Simulate async search delay
            Timer::after(Duration::from_millis(200)).await;

            // Simulate searching files/resources
            let mut results = Vec::new();

            if query.contains("file") || query.contains("doc") {
                results.push(
                    CommandPaletteItem::new("async.file1", "document.txt")
                        .category("Files")
                        .subtitle("~/Documents/")
                        .icon(IconName::FileText),
                );
                results.push(
                    CommandPaletteItem::new("async.file2", "notes.md")
                        .category("Files")
                        .subtitle("~/Documents/")
                        .icon(IconName::FileText),
                );
            }

            if query.contains("code") || query.contains("src") {
                results.push(
                    CommandPaletteItem::new("async.code1", "main.rs")
                        .category("Source")
                        .subtitle("src/")
                        .icon(IconName::Code),
                );
                results.push(
                    CommandPaletteItem::new("async.code2", "lib.rs")
                        .category("Source")
                        .subtitle("src/")
                        .icon(IconName::Code),
                );
            }

            if query.contains("test") {
                results.push(
                    CommandPaletteItem::new("async.test1", "test_utils.rs")
                        .category("Tests")
                        .subtitle("tests/")
                        .icon(IconName::TestTube),
                );
            }

            // Generic async result for any query
            results.push(
                CommandPaletteItem::new(
                    format!("async.result.{}", query),
                    format!("Async result for: {}", query),
                )
                .category("Async Search")
                .icon(IconName::Search)
                .subtitle("Dynamically loaded"),
            );

            results
        })
    }
}

impl Focusable for CommandPaletteStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandPaletteStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("command-palette-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        section("Basic Usage")
                            .child("Open a command palette with static items and fuzzy search.")
                            .child(
                                Button::new("show-static")
                                    .outline()
                                    .label("Open Static Palette")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_static_palette(window, cx)
                                    })),
                            ),
                    )
                    .child(
                        section("Async Provider")
                            .child(
                                "Command palette with static items plus async search. \
                                Try typing 'file', 'code', or 'test' to see async results.",
                            )
                            .child(
                                Button::new("show-async")
                                    .outline()
                                    .label("Open Async Palette")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_async_palette(window, cx)
                                    })),
                            ),
                    )
                    .child(
                        section("Custom Configuration")
                            .child("Command palette with custom width, height, and placeholder.")
                            .child(
                                Button::new("show-custom")
                                    .outline()
                                    .label("Open Custom Config Palette")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.show_custom_config_palette(window, cx)
                                    })),
                            ),
                    )
                    .child(section("Last Selected").child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Last selected item:"),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(
                                        self.last_selected
                                            .as_ref()
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| "None".to_string()),
                                    ),
                            ),
                    ))
                    .child(
                        section("Features")
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child("• Fuzzy search with highlighted matches")
                                    .child("• Keyboard navigation (↑/↓, Enter, Escape)")
                                    .child("• Categories and icons")
                                    .child("• Keyboard shortcuts display")
                                    .child("• Static and async item providers")
                                    .child("• Customizable appearance")
                                    .child("• Glassmorphic surface design"),
                            )
                    )
                    .child(
                        section("Keyboard Shortcuts")
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child("• ↑/↓: Navigate items")
                                    .child("• Enter: Select item")
                                    .child("• Escape: Close palette")
                                    .child("• cmd-k (or ctrl-k): Open palette (if initialized with default config)"),
                            )
                    ),
            )
    }
}

pub fn init(cx: &mut App) {
    // Initialize the command palette with default config
    CommandPalette::init(cx, CommandPaletteConfig::default());
}
