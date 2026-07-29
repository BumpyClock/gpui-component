---
title: "Local GPUI Override Workflow"
summary: "Use an uncommitted Cargo patch for coordinated framework and GPUI development."
read_when: "testing framework changes against a sibling GPUI checkout or changing workspace GPUI dependency pins"
---
# Local GPUI Override Workflow

The committed framework always uses the immutable GPUI Git revision in the
workspace manifest. The repository does not vendor GPUI and has no GPUI
submodule.

For coordinated development, clone GPUI beside this repository and create an
uncommitted `.cargo/config.toml` override:

```toml
[patch."https://github.com/BumpyClock/gpui"]
bumpyclock-gpui = { path = "../gpui/crates/gpui" }
gpui_platform = { path = "../gpui/crates/gpui_platform" }
gpui_macros = { path = "../gpui/crates/gpui_macros" }
sum_tree = { path = "../gpui/crates/sum_tree" }
```

Patch keys are Cargo package identities, not dependency aliases or Rust import
names. The framework manifest must already declare
`gpui = { package = "bumpyclock-gpui", ... }`; a patch cannot bridge an old
`gpui` package identity to the renamed package. During an identity transition,
test in a disposable framework snapshot and wait for the canonical GPUI commit
before changing the committed pin.

Add every GPUI package resolved by the framework to the patch table. Keep this
developer-specific override out of commits. Before release work, remove it and
verify the committed Git dependency:

```bash
cargo xtask compatibility check
cargo xtask release-check
```

After the GPUI change merges, update the framework's full GPUI revision and
exact registry versions together, regenerate compatibility documentation, then
repeat the checks above. Engine packages must be published before framework
packages; the committed override never represents a release dependency.
