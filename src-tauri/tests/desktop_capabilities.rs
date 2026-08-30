use std::{fs, path::PathBuf};

#[test]
fn desktop_capability_allows_live_events_and_global_bar_dragging() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("capabilities")
        .join("desktop.json");
    let document = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is required: {error}", path.display()));
    let value: serde_json::Value = serde_json::from_str(&document).expect("valid capability JSON");
    let permissions = value["permissions"]
        .as_array()
        .expect("capability permissions array");

    for required in [
        "core:event:default",
        "core:window:allow-outer-size",
        "core:window:allow-scale-factor",
        "core:window:allow-start-dragging",
        "core:window:allow-start-resize-dragging",
    ] {
        assert!(
            permissions.iter().any(|permission| permission == required),
            "desktop capability must include {required}"
        );
    }
}

#[test]
fn desktop_csp_allows_the_bundled_inline_svg_mark() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let document = fs::read_to_string(&path).expect("tauri config");
    let value: serde_json::Value = serde_json::from_str(&document).expect("valid tauri config");
    let csp = value["app"]["security"]["csp"]
        .as_str()
        .expect("desktop CSP string");

    assert!(
        csp.contains("img-src") && csp.contains("data:"),
        "Vite inlines the approved sub-1 KB SVG mark as a data URL"
    );
}

#[test]
fn desktop_bundle_uses_complete_platform_icons_generated_from_the_logo() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let document = fs::read_to_string(root.join("tauri.conf.json")).expect("tauri config");
    let value: serde_json::Value = serde_json::from_str(&document).expect("valid tauri config");
    let icons = value["bundle"]["icon"]
        .as_array()
        .expect("bundle icon array");

    for required in ["icons/icon.png", "icons/icon.icns", "icons/icon.ico"] {
        assert!(
            icons.iter().any(|icon| icon == required),
            "desktop bundle must include {required}"
        );
        assert!(root.join(required).is_file(), "{required} must exist");
    }
}
