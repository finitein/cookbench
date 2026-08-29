/// Desktop environments whose overlay behavior is part of the V1 contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopEnvironment {
    Macos,
    Windows,
    UbuntuX11,
    GnomeWayland,
}

/// Strength of the requested global and detached overlay behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlaySupport {
    Full,
    NearFull,
    BestEffort,
}

/// Capabilities deliberately distinguish a graphical UI from compositor-level
/// overlay guarantees. Wayland is still a usable graphical application.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OverlayCapabilities {
    pub graphical_ui: bool,
    pub overlay: OverlaySupport,
    pub detached_overlay: OverlaySupport,
    pub optional_extension_available: bool,
}

pub const fn capabilities_for(
    environment: DesktopEnvironment,
    gnome_extension_installed: bool,
) -> OverlayCapabilities {
    match environment {
        DesktopEnvironment::Macos | DesktopEnvironment::Windows => OverlayCapabilities {
            graphical_ui: true,
            overlay: OverlaySupport::Full,
            detached_overlay: OverlaySupport::Full,
            optional_extension_available: false,
        },
        DesktopEnvironment::UbuntuX11 => OverlayCapabilities {
            graphical_ui: true,
            overlay: OverlaySupport::NearFull,
            detached_overlay: OverlaySupport::NearFull,
            optional_extension_available: false,
        },
        DesktopEnvironment::GnomeWayland if gnome_extension_installed => OverlayCapabilities {
            graphical_ui: true,
            overlay: OverlaySupport::Full,
            detached_overlay: OverlaySupport::Full,
            optional_extension_available: true,
        },
        DesktopEnvironment::GnomeWayland => OverlayCapabilities {
            graphical_ui: true,
            overlay: OverlaySupport::BestEffort,
            detached_overlay: OverlaySupport::BestEffort,
            optional_extension_available: true,
        },
    }
}

/// Detect the active platform conservatively. GNOME Wayland is kept separate
/// because the compositor, not Cookbench, decides whether a window stays above
/// other applications.
pub fn current_desktop_environment() -> DesktopEnvironment {
    #[cfg(target_os = "macos")]
    {
        DesktopEnvironment::Macos
    }

    #[cfg(target_os = "windows")]
    {
        DesktopEnvironment::Windows
    }

    #[cfg(target_os = "linux")]
    {
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        if session_type.eq_ignore_ascii_case("wayland")
            && desktop.to_ascii_uppercase().contains("GNOME")
        {
            DesktopEnvironment::GnomeWayland
        } else {
            DesktopEnvironment::UbuntuX11
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        DesktopEnvironment::UbuntuX11
    }
}
