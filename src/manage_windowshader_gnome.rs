//! GNOME Shell Windowshader position reporting (normal desktop session only).
//! The GNOME extension reports compositor-owned coordinates via session D-Bus.
//! Both libdecor and XDG use this same controller; neither guesses global X/Y.

use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct PositionReceiver(Arc<Mutex<Option<(i32, i32)>>>);

#[zbus::interface(name = "org.screenshaver.Windowshader.Position")]
impl PositionReceiver {
    fn report(&self, x: i32, y: i32) {
        // GNOME logical coordinates can be negative on multi-monitor layouts.
        if let Ok(mut position) = self.0.lock() {
            *position = Some((x, y));
        }
    }
}

pub(crate) fn is_gnome_wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland")
        && std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop.eq_ignore_ascii_case("GNOME"))
}

pub(crate) struct WindowshaderGnomeGuard {
    position: Arc<Mutex<Option<(i32, i32)>>>,
    _connection: zbus::blocking::Connection,
    _runtime: tokio::runtime::Runtime,
}

impl WindowshaderGnomeGuard {
    pub(crate) fn start() -> Result<Option<Self>, String> {
        if !is_gnome_wayland_session() {
            return Ok(None);
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|error| format!("GNOME geometry D-Bus runtime: {error}"))?;
        let position = Arc::new(Mutex::new(None));
        let connection = {
            let _entered = runtime.enter();
            let connection = zbus::blocking::Connection::session()
                .map_err(|error| format!("GNOME geometry session bus: {error}"))?;
            connection.request_name("org.screenshaver.Windowshader.Gnome")
                .map_err(|error| format!("GNOME geometry bus name: {error}"))?;
            connection.object_server().at("/Windowshader", PositionReceiver(position.clone()))
                .map_err(|error| format!("GNOME geometry D-Bus object: {error}"))?;
            connection
        };
        eprintln!("[GNOME GEOMETRY] position receiver ready");
        Ok(Some(Self { position, _connection: connection, _runtime: runtime }))
    }

    pub(crate) fn last_position(&self) -> Option<(i32, i32)> {
        self.position.lock().ok().and_then(|position| *position)
    }
}
