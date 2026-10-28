//! GNOME Windowshader integration boundary.
//!
//! Stage 1 is read-only: the GNOME Shell extension reports compositor-side
//! geometry to the Shell journal. No geometry is changed or persisted here.
//! Both libdecor and XDG windowshaders are identified by the same app identity.
//!
//! Future stages may add an authenticated session-scoped D-Bus handshake and
//! coordinate persistence. Do not infer desktop X/Y from Wayland surface-local
//! coordinates; only the compositor can report global placement.

/// Whether this session is eligible for GNOME-specific Windowshader handling.
/// This does not imply the Shell extension is installed or enabled.
#[allow(dead_code)]
pub(crate) fn is_gnome_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland")
        && std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop.eq_ignore_ascii_case("GNOME"))
}
