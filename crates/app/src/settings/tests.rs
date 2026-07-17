use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_component_storage::PathLayout;
use tempfile::TempDir;

use super::*;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
struct TestSettings {
    value: String,
    stage: u32,
}

impl AppSettings for TestSettings {
    const SCHEMA_VERSION: u32 = 3;
}

struct TempPaths {
    paths: AppPaths,
    _root: TempDir,
}

fn temp_paths() -> TempPaths {
    let probe = AppPaths::new(
        "gpui-settings-probe",
        PathLayout::SingleRoot(".gpui-settings-probe".to_string()),
    )
    .unwrap();
    let home = probe
        .config_dir()
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap();
    let root = tempfile::Builder::new()
        .prefix(".gpui-settings-")
        .tempdir_in(home)
        .unwrap();
    let root_name = root.path().file_name().unwrap().to_str().unwrap();
    let paths = AppPaths::new(
        "gpui-settings-test",
        PathLayout::SingleRoot(root_name.to_string()),
    )
    .unwrap();
    assert_eq!(paths.config_dir(), root.path().join("config"));
    TempPaths { paths, _root: root }
}

static MIGRATION_STEPS: AtomicUsize = AtomicUsize::new(0);

fn migrate_1_to_2(mut raw: toml::Value) -> Result<toml::Value, String> {
    assert_eq!(raw["value"].as_str(), Some("one"));
    raw["stage"] = toml::Value::Integer(2);
    MIGRATION_STEPS.fetch_add(1, Ordering::SeqCst);
    Ok(raw)
}

fn migrate_2_to_3(mut raw: toml::Value) -> Result<toml::Value, String> {
    assert_eq!(raw["stage"].as_integer(), Some(2));
    raw["stage"] = toml::Value::Integer(3);
    MIGRATION_STEPS.fetch_add(1, Ordering::SeqCst);
    Ok(raw)
}

#[test]
fn migration_chain_runs_each_intermediate_step() {
    MIGRATION_STEPS.store(0, Ordering::SeqCst);
    let temp = temp_paths();
    std::fs::create_dir_all(temp.paths.config_dir()).unwrap();
    std::fs::write(
        temp.paths.config_dir().join("settings.toml"),
        "schema_version = 1\nvalue = \"one\"\nstage = 1\n",
    )
    .unwrap();

    let entry = SettingsPlugin::<TestSettings>::new(StoreKey::PRIMARY)
        .migrate(1, 2, migrate_1_to_2)
        .migrate(2, 3, migrate_2_to_3)
        .open_entry(&temp.paths)
        .unwrap();

    assert_eq!(MIGRATION_STEPS.load(Ordering::SeqCst), 2);
    assert_eq!(entry.value.stage, 3);
    entry.store.flush().unwrap();
}

#[test]
fn future_version_refuses_update_and_preserves_bytes() {
    let temp = temp_paths();
    std::fs::create_dir_all(temp.paths.config_dir()).unwrap();
    let path = temp.paths.config_dir().join("settings.toml");
    let bytes = b"schema_version = 99\nvalue = \"future\"\nstage = 99\n";
    std::fs::write(&path, bytes).unwrap();

    let mut entry = SettingsPlugin::<TestSettings>::new(StoreKey::PRIMARY)
        .open_entry(&temp.paths)
        .unwrap();
    let closure_ran = std::cell::Cell::new(false);
    let result = entry.snapshot_for_update().and_then(|previous| {
        closure_ran.set(true);
        entry.value.value = "changed".to_string();
        entry.finish_update(previous)
    });
    assert!(matches!(
        result,
        Err(SettingsError::UnsupportedFutureVersion {
            found: 99,
            supported: 3
        })
    ));
    assert!(
        !closure_ran.get(),
        "refused update must not invoke callback"
    );
    assert_eq!(entry.value, TestSettings::default());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn same_type_in_two_named_stores_has_separate_files_and_state() {
    let temp = temp_paths();
    let first_key = StoreKey::new("first").unwrap();
    let second_key = StoreKey::new("second").unwrap();
    let mut first = SettingsPlugin::<TestSettings>::new(first_key.clone())
        .open_entry(&temp.paths)
        .unwrap();
    let mut second = SettingsPlugin::<TestSettings>::new(second_key.clone())
        .open_entry(&temp.paths)
        .unwrap();
    first.value.value = "first".to_string();
    second.value.value = "second".to_string();
    first.queue_current().unwrap();
    second.queue_current().unwrap();
    first.store.flush().unwrap();
    second.store.flush().unwrap();

    assert_ne!(first.value, second.value);
    assert!(temp.paths.config_dir().join(first_key.filename()).exists());
    assert!(temp.paths.config_dir().join(second_key.filename()).exists());
}

#[test]
fn corrupt_file_loads_default() {
    let temp = temp_paths();
    std::fs::create_dir_all(temp.paths.config_dir()).unwrap();
    let path = temp.paths.config_dir().join("settings.toml");
    std::fs::write(&path, "not = valid = toml").unwrap();

    let entry = SettingsPlugin::<TestSettings>::new(StoreKey::PRIMARY)
        .open_entry(&temp.paths)
        .unwrap();

    assert_eq!(entry.value, TestSettings::default());
    assert!(!path.exists(), "storage must archive corrupt primary");
}

#[test]
fn update_flush_reload_roundtrip() {
    let temp = temp_paths();
    let mut entry = SettingsPlugin::<TestSettings>::new(StoreKey::PRIMARY)
        .open_entry(&temp.paths)
        .unwrap();
    let previous = entry.snapshot_for_update().unwrap();
    entry.value.value = "persisted".to_string();
    entry.value.stage = 3;
    entry.finish_update(previous).unwrap();
    ErasedSettingsEntry::flush(&mut entry).unwrap();
    drop(entry);

    let reloaded = SettingsPlugin::<TestSettings>::new(StoreKey::PRIMARY)
        .open_entry(&temp.paths)
        .unwrap();
    assert_eq!(
        reloaded.value,
        TestSettings {
            value: "persisted".to_string(),
            stage: 3,
        }
    );
}
