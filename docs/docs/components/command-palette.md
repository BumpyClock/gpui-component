---
title: Command Palette
description: A modal command palette with fuzzy search, keyboard navigation, and customizable appearance.
---

# Command Palette

A powerful command palette component with fuzzy search, keyboard navigation, categories, and async item providers. Features a glassmorphic surface design and requires the dialog layer for display.

## Import

```rust
use gpui_component::command_palette::{
    CommandPalette, CommandPaletteConfig, CommandPaletteItem,
    CommandPaletteProvider, StaticProvider, CommandPaletteEvent
};
```

## Setup

### Initialize at Startup

Call `CommandPalette::init` once during application startup to register the component and set up the global keyboard shortcut:

```rust
fn main() {
    // Initialize with default config (cmd-k on macOS, ctrl-k elsewhere)
    CommandPalette::init(cx, CommandPaletteConfig::default());
}
```

### Setup Dialog Layer

The Command Palette requires the dialog layer to be rendered in your root view. See the [Dialog documentation](dialog#setup-application-root-view-for-display-of-dialogs) for details on setting up `Root::render_dialog_layer`.

```rust
impl Render for MyApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        div()
            .size_full()
            .child(self.view.clone())
            .children(dialog_layer)
    }
}
```

## Opening the Palette

### With Static Items

```rust
use std::sync::Arc;

let items = vec![
    CommandPaletteItem::new("file.open", "Open File")
        .category("File")
        .shortcut("cmd-o"),
    CommandPaletteItem::new("file.save", "Save File")
        .category("File")
        .shortcut("cmd-s"),
    CommandPaletteItem::new("edit.undo", "Undo")
        .category("Edit")
        .shortcut("cmd-z"),
];

let provider = Arc::new(StaticProvider::new(items));
let handle = CommandPalette::open(window, cx, provider);
```

### Subscribe to Selection Events

```rust
let handle = CommandPalette::open(window, cx, provider);

cx.subscribe(&handle.state(), move |_, event, _, cx| {
    match event {
        CommandPaletteEvent::Selected { item } => {
            println!("Selected: {}", item.title);
            // Handle the selected item
        }
        CommandPaletteEvent::Dismissed => {
            println!("Palette dismissed");
        }
    }
}).detach();
```

### Close Programmatically

```rust
let handle = CommandPalette::open(window, cx, provider);

// Later, close the palette
handle.close(window, cx);
```

## Item Configuration

### Basic Item

```rust
let item = CommandPaletteItem::new("item-id", "Item Title");
```

### Item with All Options

```rust
let item = CommandPaletteItem::new("git.commit", "Commit Changes")
    .category("Git")
    .subtitle("Stage and commit your changes")
    .icon(IconName::GitCommit)
    .shortcut("cmd-shift-c")
    .keyword("vcs")
    .keyword("version control")
    .disabled(false);
```

### Items with Icons and Categories

```rust
let items = vec![
    CommandPaletteItem::new("search.files", "Search Files")
        .category("Search")
        .icon(IconName::Search)
        .shortcut("cmd-p"),

    CommandPaletteItem::new("git.status", "Git Status")
        .category("Git")
        .icon(IconName::GitBranch)
        .shortcut("cmd-shift-g"),
];
```

## Custom Provider

Implement `CommandPaletteProvider` to create custom providers with static and/or async items:

```rust
use gpui::Task;
use smol::Timer;
use std::time::Duration;

struct MyProvider {
    static_items: Vec<CommandPaletteItem>,
}

impl CommandPaletteProvider for MyProvider {
    fn items(&self, _cx: &App) -> Vec<CommandPaletteItem> {
        self.static_items.clone()
    }

    fn query(&self, query: &str, cx: &App) -> Task<Vec<CommandPaletteItem>> {
        let query = query.to_string();

        cx.background_spawn(async move {
            // Simulate async search (e.g., search files, API call)
            Timer::after(Duration::from_millis(100)).await;

            vec![
                CommandPaletteItem::new("async-1", format!("Result for: {}", query))
                    .category("Async"),
            ]
        })
    }
}
```

### Using Custom Provider

```rust
let provider = Arc::new(MyProvider {
    static_items: vec![
        CommandPaletteItem::new("static-1", "Static Item")
            .category("Static"),
    ],
});

let handle = CommandPalette::open(window, cx, provider);
```

## Matcher Selection

Choose different fuzzy matching algorithms:

```rust
use gpui_component::command_palette::{CommandMatcherKind, NucleoMatcher};

let config = CommandPaletteConfig {
    matcher: CommandMatcherKind::Nucleo,  // Default: async-friendly
    // or
    matcher: CommandMatcherKind::FuzzyMatcher,  // SkimMatcherV2
    ..Default::default()
};

CommandPalette::init(cx, config);
```

### Custom Matcher

```rust
use gpui_component::command_palette::{CommandMatcher, CommandPaletteMatch};

struct MyMatcher;

impl CommandMatcher for MyMatcher {
    fn match_item(&self, query: &str, item: &CommandPaletteItem) -> Option<CommandPaletteMatch> {
        // Custom matching logic
        if item.title.contains(query) {
            Some(CommandPaletteMatch::new(100))
        } else {
            None
        }
    }
}

let config = CommandPaletteConfig {
    matcher: CommandMatcherKind::Custom(Arc::new(MyMatcher)),
    ..Default::default()
};
```

## Customization

### Custom Configuration

```rust
let config = CommandPaletteConfig {
    shortcut: Some("cmd-shift-p".into()),  // Custom shortcut
    placeholder: "Search commands...".into(),
    width: 640.0,
    max_height: 500.0,
    max_results: 100,
    show_footer: true,
    show_categories_inline: true,
    matcher: CommandMatcherKind::Nucleo,
};

CommandPalette::init(cx, config);
```

### Per-Instance Configuration

```rust
let custom_config = CommandPaletteConfig {
    width: 800.0,
    placeholder: "Search actions...".into(),
    ..Default::default()
};

let handle = CommandPalette::open_with_config(
    window,
    cx,
    provider,
    custom_config
);
```

### Disable Global Shortcut

```rust
let config = CommandPaletteConfig {
    shortcut: None,  // Disable default keyboard shortcut
    ..Default::default()
};

CommandPalette::init(cx, config);
```

## Examples

### File Opener

```rust
let items = vec![
    CommandPaletteItem::new("recent-1", "src/main.rs")
        .category("Recent Files")
        .icon(IconName::File),
    CommandPaletteItem::new("recent-2", "README.md")
        .category("Recent Files")
        .icon(IconName::FileText),
];

let provider = Arc::new(StaticProvider::new(items));
let handle = CommandPalette::open(window, cx, provider);

cx.subscribe(&handle.state(), |this, event, window, cx| {
    if let CommandPaletteEvent::Selected { item } = event {
        // Open the selected file
        this.open_file(&item.id, window, cx);
    }
}).detach();
```

### Git Commands

```rust
let items = vec![
    CommandPaletteItem::new("git.commit", "Commit Changes")
        .category("Git")
        .icon(IconName::GitCommit)
        .shortcut("cmd-shift-c"),

    CommandPaletteItem::new("git.push", "Push to Remote")
        .category("Git")
        .icon(IconName::Upload)
        .shortcut("cmd-shift-p"),

    CommandPaletteItem::new("git.pull", "Pull from Remote")
        .category("Git")
        .icon(IconName::Download),
];
```

### Mixed Static and Async

```rust
struct HybridProvider {
    commands: Vec<CommandPaletteItem>,
}

impl CommandPaletteProvider for HybridProvider {
    // Static commands always available
    fn items(&self, _cx: &App) -> Vec<CommandPaletteItem> {
        self.commands.clone()
    }

    // Dynamic file search
    fn query(&self, query: &str, cx: &App) -> Task<Vec<CommandPaletteItem>> {
        let q = query.to_string();
        cx.background_spawn(async move {
            // Search filesystem
            search_files(&q).await
        })
    }
}
```

## Notes

- **Dialog Layer Required**: The Command Palette renders in the dialog layer. Ensure your root view includes `Root::render_dialog_layer`.
- **Glassmorphic Surface**: The palette uses a modern glassmorphic design with blur and transparency effects.
- **Keyboard Navigation**: Use arrow keys (↑/↓) to navigate, Enter to select, and Escape to dismiss.
- **Async Support**: Providers can implement async queries that merge with static items. Async items override static items with the same ID.
- **Fuzzy Matching**: Both Nucleo (default) and SkimMatcherV2 matchers provide fuzzy search with highlighted matches.
- **Categories**: Items can be grouped by category for better organization.
- **Performance**: Results are limited by `max_results` (default: 50) for optimal rendering performance.
