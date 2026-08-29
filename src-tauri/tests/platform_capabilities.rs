use cookbench_desktop_lib::platform::{capabilities_for, DesktopEnvironment, OverlaySupport};

#[test]
fn macos_supports_full_overlay() {
    let caps = capabilities_for(DesktopEnvironment::Macos, false);
    assert_eq!(caps.overlay, OverlaySupport::Full);
    assert!(caps.graphical_ui);
}

#[test]
fn windows_supports_full_overlay_without_extension() {
    let caps = capabilities_for(DesktopEnvironment::Windows, false);
    assert_eq!(caps.overlay, OverlaySupport::Full);
    assert!(caps.graphical_ui);
    assert!(!caps.optional_extension_available);
}

#[test]
fn ubuntu_x11_reports_near_full_overlay() {
    let caps = capabilities_for(DesktopEnvironment::UbuntuX11, false);
    assert_eq!(caps.overlay, OverlaySupport::NearFull);
    assert!(caps.graphical_ui);
}

#[test]
fn wayland_without_extension_reports_best_effort_overlay() {
    let caps = capabilities_for(DesktopEnvironment::GnomeWayland, false);
    assert_eq!(caps.overlay, OverlaySupport::BestEffort);
    assert!(caps.graphical_ui);
    assert!(caps.optional_extension_available);
}

#[test]
fn wayland_with_extension_reports_full_overlay() {
    let caps = capabilities_for(DesktopEnvironment::GnomeWayland, true);
    assert_eq!(caps.overlay, OverlaySupport::Full);
    assert!(caps.graphical_ui);
}
