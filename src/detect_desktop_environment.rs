//! detect_desktop_environment.rs
//!
//! Detects the desktop environment for backend-selection decisions.
//!
//! This is intentionally separate from session/idle backend detection.
//! A Wayland session, for example, may be running KDE Plasma, GNOME, or
//! another desktop environment.
//!
//! Detection is based only on the current process environment. The module
//! performs no D-Bus calls, filesystem probing, or compositor interaction.

use std::env;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum DesktopEnvironment {
    KdePlasma,
    Gnome,
    Xfce,
    Other,
    Unknown,
}

impl DesktopEnvironment {
    pub fn name(
        self,
    ) -> &'static str {
        match self {
            Self::KdePlasma => {
                "KDE Plasma"
            }

            Self::Gnome => {
                "GNOME"
            }

            Self::Xfce => {
                "XFCE"
            }

            Self::Other => {
                "Other"
            }

            Self::Unknown => {
                "Unknown"
            }
        }
    }

    pub fn is_kde_plasma(
        self,
    ) -> bool {
        self == Self::KdePlasma
    }

    pub fn is_gnome(
        self,
    ) -> bool {
        self == Self::Gnome
    }

    pub fn is_xfce(
        self,
    ) -> bool {
        self == Self::Xfce
    }
}

/// Detects the current desktop environment.
///
/// Primary XDG indicators:
///
///   XDG_CURRENT_DESKTOP
///   XDG_SESSION_DESKTOP
///
/// Common compatibility indicators are also considered:
///
///   DESKTOP_SESSION
///   KDE_FULL_SESSION
///   GNOME_DESKTOP_SESSION_ID
///
/// Matching is case-insensitive. `XDG_CURRENT_DESKTOP` may contain multiple
/// desktop identifiers separated by `:` or `;`, so each component is checked
/// independently.
pub fn detect(
) -> DesktopEnvironment {
    if environment_variable_contains_desktop(
        "XDG_CURRENT_DESKTOP",
        is_kde_identifier,
    ) {
        return DesktopEnvironment::KdePlasma;
    }

    if environment_variable_contains_desktop(
        "XDG_SESSION_DESKTOP",
        is_kde_identifier,
    ) {
        return DesktopEnvironment::KdePlasma;
    }

    if environment_variable_contains_desktop(
        "DESKTOP_SESSION",
        is_kde_identifier,
    ) {
        return DesktopEnvironment::KdePlasma;
    }

    if environment_variable_is_true(
        "KDE_FULL_SESSION"
    ) {
        return DesktopEnvironment::KdePlasma;
    }

    if environment_variable_contains_desktop(
        "XDG_CURRENT_DESKTOP",
        is_gnome_identifier,
    ) {
        return DesktopEnvironment::Gnome;
    }

    if environment_variable_contains_desktop(
        "XDG_SESSION_DESKTOP",
        is_gnome_identifier,
    ) {
        return DesktopEnvironment::Gnome;
    }

    if environment_variable_contains_desktop(
        "DESKTOP_SESSION",
        is_gnome_identifier,
    ) {
        return DesktopEnvironment::Gnome;
    }

    if env::var_os(
        "GNOME_DESKTOP_SESSION_ID"
    )
    .is_some()
    {
        return DesktopEnvironment::Gnome;
    }

    if environment_variable_contains_desktop(
        "XDG_CURRENT_DESKTOP",
        is_xfce_identifier,
    ) {
        return DesktopEnvironment::Xfce;
    }

    if environment_variable_contains_desktop(
        "XDG_SESSION_DESKTOP",
        is_xfce_identifier,
    ) {
        return DesktopEnvironment::Xfce;
    }

    if environment_variable_contains_desktop(
        "DESKTOP_SESSION",
        is_xfce_identifier,
    ) {
        return DesktopEnvironment::Xfce;
    }

    if has_any_desktop_marker() {
        DesktopEnvironment::Other
    } else {
        DesktopEnvironment::Unknown
    }
}

fn environment_variable_contains_desktop(
    variable: &str,
    predicate: fn(&str) -> bool,
) -> bool {
    let Some(value) =
        env::var_os(variable)
    else {
        return false;
    };

    let value =
        value.to_string_lossy();

    value
        .split(
            |character| {
                character == ':'
                    || character == ';'
            }
        )
        .map(str::trim)
        .filter(
            |component| {
                !component.is_empty()
            }
        )
        .any(predicate)
}

fn environment_variable_is_true(
    variable: &str,
) -> bool {
    let Some(value) =
        env::var_os(variable)
    else {
        return false;
    };

    matches!(
        value
            .to_string_lossy()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1"
            | "true"
            | "yes"
            | "on"
    )
}

fn is_kde_identifier(
    value: &str,
) -> bool {
    let normalized =
        normalize_identifier(value);

    normalized == "kde"
        || normalized == "plasma"
        || normalized.starts_with(
            "plasma-"
        )
        || normalized.starts_with(
            "plasma_"
        )
        || normalized.contains(
            "kde-plasma"
        )
        || normalized.contains(
            "kde_plasma"
        )
}

fn is_gnome_identifier(
    value: &str,
) -> bool {
    let normalized =
        normalize_identifier(value);

    normalized == "gnome"
        || normalized.starts_with(
            "gnome-"
        )
        || normalized.starts_with(
            "gnome_"
        )
        || normalized.contains(
            "ubuntu:gnome"
        )
}

fn is_xfce_identifier(
    value: &str,
) -> bool {
    let normalized =
        normalize_identifier(value);

    normalized == "xfce"
        || normalized == "xfce4"
        || normalized.starts_with(
            "xfce-"
        )
        || normalized.starts_with(
            "xfce_"
        )
}

fn normalize_identifier(
    value: &str,
) -> String {
    value
        .trim()
        .to_ascii_lowercase()
}

fn has_any_desktop_marker(
) -> bool {
    [
        "XDG_CURRENT_DESKTOP",
        "XDG_SESSION_DESKTOP",
        "DESKTOP_SESSION",
        "KDE_FULL_SESSION",
        "GNOME_DESKTOP_SESSION_ID",
    ]
    .iter()
    .any(
        |variable| {
            env::var_os(variable)
                .is_some()
        }
    )
}

#[cfg(test)]
mod tests {
    use super::{
        is_gnome_identifier,
        is_kde_identifier,
        is_xfce_identifier,
    };

    #[test]
    fn recognizes_kde_identifiers(
    ) {
        assert!(
            is_kde_identifier(
                "KDE"
            )
        );

        assert!(
            is_kde_identifier(
                "plasma"
            )
        );

        assert!(
            is_kde_identifier(
                "plasma-wayland"
            )
        );

        assert!(
            is_kde_identifier(
                "KDE-Plasma"
            )
        );
    }

    #[test]
    fn rejects_non_kde_identifiers(
    ) {
        assert!(
            !is_kde_identifier(
                "GNOME"
            )
        );

        assert!(
            !is_kde_identifier(
                "XFCE"
            )
        );
    }

    #[test]
    fn recognizes_gnome_identifiers(
    ) {
        assert!(
            is_gnome_identifier(
                "GNOME"
            )
        );

        assert!(
            is_gnome_identifier(
                "gnome-wayland"
            )
        );

        assert!(
            is_gnome_identifier(
                "gnome-classic"
            )
        );
    }

    #[test]
    fn rejects_non_gnome_identifiers(
    ) {
        assert!(
            !is_gnome_identifier(
                "KDE"
            )
        );

        assert!(
            !is_gnome_identifier(
                "XFCE"
            )
        );
    }

    #[test]
    fn recognizes_xfce_identifiers(
    ) {
        assert!(
            is_xfce_identifier(
                "XFCE"
            )
        );

        assert!(
            is_xfce_identifier(
                "xfce4"
            )
        );

        assert!(
            is_xfce_identifier(
                "xfce-session"
            )
        );
    }

    #[test]
    fn rejects_non_xfce_identifiers(
    ) {
        assert!(
            !is_xfce_identifier(
                "GNOME"
            )
        );

        assert!(
            !is_xfce_identifier(
                "KDE"
            )
        );
    }

}
