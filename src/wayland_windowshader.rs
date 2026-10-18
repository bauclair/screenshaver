//! Windowshader-owned Wayland geometry and presentation-state persistence.
//! The native XDG/EGL loop remains in wayland_wallpaper during incremental extraction.

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct WindowshaderGeometry {
    #[serde(default)]
    pub(crate) x: Option<i32>,
    #[serde(default)]
    pub(crate) y: Option<i32>,
    pub(crate) width: i32,
    pub(crate) height: i32,
    #[serde(default)]
    pub(crate) maximized: bool,
}

impl Default for WindowshaderGeometry {
    fn default() -> Self {
        Self { x: None, y: None, width: 960, height: 540, maximized: false }
    }
}

pub(crate) fn load_windowshader_geometry() -> WindowshaderGeometry {
    let default = WindowshaderGeometry::default();
    let geometry = crate::manage_runtime_state::read_state().ok()
        .and_then(|root| root.get("windowshader").cloned())
        .and_then(|value| serde_json::from_value::<WindowshaderGeometry>(value).ok());
    match geometry {
        Some(geometry) if (64..=16384).contains(&geometry.width)
            && (64..=16384).contains(&geometry.height) => geometry,
        _ => default,
    }
}

pub(crate) fn save_windowshader_geometry(geometry: WindowshaderGeometry) -> Result<(), String> {
    let value = serde_json::to_value(geometry).map_err(|error| error.to_string())?;
    crate::manage_runtime_state::update_state(move |root| {
        root.as_object_mut().ok_or("Runtime state root is not an object")?
            .insert("windowshader".to_string(), value);
        Ok(())
    })
}

