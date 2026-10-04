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
    Lxde,
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

            Self::Lxde => {
                "LXDE"
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

    pub fn is_lxde(self) -> bool {
        self == Self::Lxde
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

    for variable in ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION"] {
        if environment_variable_contains_desktop(variable, is_lxde_identifier) {
            return DesktopEnvironment::Lxde;
        }
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

fn is_lxde_identifier(value: &str) -> bool {
    let normalized = normalize_identifier(value);
    normalized == "lxde" || normalized.starts_with("lxde-")
        || normalized.starts_with("lxde_")
}

/// Detect PCManFM's actual X11 desktop window. Check EWMH's client list,
/// then walk the X11 window hierarchy (some window managers omit desktop
/// windows from the client list or reparent them).
pub fn pcmanfm_manages_x11_desktop() -> bool {
    use std::ffi::{CStr, CString};
    use std::ptr;
    use x11::xlib;

    unsafe fn matches_desktop(
        display: *mut xlib::Display,
        window: xlib::Window,
        type_atom: xlib::Atom,
        desktop_atom: xlib::Atom,
    ) -> bool {
        let mut hint: xlib::XClassHint = std::mem::zeroed();
        if xlib::XGetClassHint(display, window, &mut hint) == 0 {
            return false;
        }
        let pcmanfm = [hint.res_name, hint.res_class].iter().any(|&value| {
            !value.is_null()
                && CStr::from_ptr(value).to_string_lossy()
                    .eq_ignore_ascii_case("pcmanfm")
        });
        if !hint.res_name.is_null() { xlib::XFree(hint.res_name as *mut _); }
        if !hint.res_class.is_null() { xlib::XFree(hint.res_class as *mut _); }
        if !pcmanfm { return false; }

        let mut actual_type = 0;
        let mut actual_format = 0;
        let mut count = 0;
        let mut remaining = 0;
        let mut property: *mut u8 = ptr::null_mut();
        let status = xlib::XGetWindowProperty(
            display, window, type_atom, 0, 32, xlib::False, xlib::XA_ATOM,
            &mut actual_type, &mut actual_format, &mut count,
            &mut remaining, &mut property,
        );
        let matched = status == xlib::Success as i32
            && actual_type == xlib::XA_ATOM
            && actual_format == 32
            && !property.is_null()
            && std::slice::from_raw_parts(property as *const xlib::Atom, count as usize)
                .contains(&desktop_atom);
        if !property.is_null() { xlib::XFree(property as *mut _); }
        matched
    }

    unsafe fn search_tree(
        display: *mut xlib::Display,
        window: xlib::Window,
        type_atom: xlib::Atom,
        desktop_atom: xlib::Atom,
        depth: usize,
    ) -> bool {
        if matches_desktop(display, window, type_atom, desktop_atom) { return true; }
        if depth == 0 { return false; }
        let mut root = 0;
        let mut parent = 0;
        let mut children: *mut xlib::Window = ptr::null_mut();
        let mut count = 0;
        if xlib::XQueryTree(display, window, &mut root, &mut parent,
            &mut children, &mut count) == 0 { return false; }
        let mut found = false;
        for index in 0..count {
            if search_tree(display, *children.add(index as usize),
                type_atom, desktop_atom, depth - 1) {
                found = true;
                break;
            }
        }
        if !children.is_null() { xlib::XFree(children as *mut _); }
        found
    }

    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() { return false; }
        let type_name = CString::new("_NET_WM_WINDOW_TYPE").unwrap();
        let desktop_name = CString::new("_NET_WM_WINDOW_TYPE_DESKTOP").unwrap();
        let client_name = CString::new("_NET_CLIENT_LIST").unwrap();
        let type_atom = xlib::XInternAtom(display, type_name.as_ptr(), xlib::True);
        let desktop_atom = xlib::XInternAtom(display, desktop_name.as_ptr(), xlib::True);
        let client_atom = xlib::XInternAtom(display, client_name.as_ptr(), xlib::True);
        let mut found = false;
        if type_atom != 0 && desktop_atom != 0 {
            let root = xlib::XDefaultRootWindow(display);
            if client_atom != 0 {
                let mut actual_type = 0;
                let mut actual_format = 0;
                let mut count = 0;
                let mut remaining = 0;
                let mut property: *mut u8 = ptr::null_mut();
                let status = xlib::XGetWindowProperty(display, root, client_atom,
                    0, 4096, xlib::False, xlib::XA_WINDOW,
                    &mut actual_type, &mut actual_format, &mut count,
                    &mut remaining, &mut property);
                if status == xlib::Success as i32 && actual_type == xlib::XA_WINDOW
                    && actual_format == 32 && !property.is_null() {
                    for &window in std::slice::from_raw_parts(
                        property as *const xlib::Window, count as usize) {
                        if matches_desktop(display, window, type_atom, desktop_atom) {
                            found = true;
                            break;
                        }
                    }
                }
                if !property.is_null() { xlib::XFree(property as *mut _); }
            }
            if !found { found = search_tree(display, root, type_atom, desktop_atom, 4); }
        }
        xlib::XCloseDisplay(display);
        found
    }
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
        is_lxde_identifier,
        is_xfce_identifier,
    };

    #[test]
    fn recognizes_lxde_identifiers() {
        assert!(is_lxde_identifier("LXDE"));
        assert!(is_lxde_identifier("lxde-pi"));
        assert!(!is_lxde_identifier("LXQt"));
        assert!(!is_lxde_identifier("XFCE"));
    }

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
