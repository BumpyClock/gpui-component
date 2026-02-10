# GPUI Submodule Workflow

Date: 2026-02-10

## Why

- Keep GPUI source local for patching/review.
- Keep workspace build stable with upstream Zed workspace dependency model.

## Current Setup

- Submodule path: `vendor/gpui`
- Submodule remote: `https://github.com/BumpyClock/zed`
- Workspace `gpui` dependency stays git+rev in `Cargo.toml`.

## Patch Flow

1. Edit GPUI in submodule:
   - `cd vendor/gpui`
   - create branch
   - patch + commit
   - push to `BumpyClock/zed`
2. Bump gpui rev in workspace:
   - copy new GPUI commit SHA
   - update `Cargo.toml` `workspace.dependencies.gpui.rev`
3. Pin submodule to same SHA:
   - `git -C vendor/gpui checkout <sha>`
4. Verify:
   - `cargo check -p gpui-component`
   - optional: `cargo test -p gpui-component --lib`

## Important Note

- Direct path dependency to `vendor/gpui/crates/gpui` fails because `gpui` inherits many `workspace.dependencies` from Zed workspace.
- Keep git dependency + local submodule in sync instead of path override.
