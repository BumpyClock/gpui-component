gpui_component_manifest::include_identity!();

fn main() {
    assert_eq!(APP_IDENTITY.app_id, "com.example.downstreamfixture");
    assert_eq!(APP_IDENTITY.version, env!("CARGO_PKG_VERSION"));
    println!(
        "IDENTITY_OK {} {}",
        APP_IDENTITY.app_id, APP_IDENTITY.version
    );
}
