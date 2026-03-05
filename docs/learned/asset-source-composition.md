# Asset Source Composition

read_when: changing asset packaging, adding bundled component assets, or updating downstream setup docs

- `gpui-component-assets` owns bundled component assets beyond icons, including `surface/NoiseAsset_256.png`.
- Downstream apps with custom assets should compose their `AssetSource` with `gpui_component_assets::Assets` instead of replacing it.
- Preferred pattern: app assets first, bundled component assets second via `gpui_component_assets::chain(app_assets, gpui_component_assets::Assets)`.
- Keep bundled component asset paths namespaced and stable, for example `surface/...`; do not add bare-path compatibility aliases.
