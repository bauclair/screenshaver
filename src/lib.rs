// Screenshaver KDE renderer shared-library root.
//
// This is intentionally NOT the Screenshaver application module tree. It
// contains only the renderer/policy/configuration dependency graph required
// to construct FrameRenderEngine inside KScreenLocker. Host runtimes such as
// X11/GLX wallpaper, Wayland wallpaper, session management, tray, editors, and
// screen-lock management are deliberately excluded.

mod load_config;
mod logger;
mod parse_mode;
mod parse_interval;
mod parse_subtitle_placement;
mod parse_texture_specification;

mod define_constants;
mod define_wallpaper;
mod locate_paths;

mod audio_backend;
mod analyze_audio;

mod manage_configuration;
mod manage_shader;
mod manage_textures;
mod manage_policies;

mod classify_shader;
mod isf_types;
mod parse_isf;
mod preprocess_isf;
mod apply_shader_inputs;
mod preprocess_shader;
mod load_shader;
mod load_shader_source;
mod compile_shader;

mod generate_bricks;
mod generate_cellular;
mod generate_clouds;
mod generate_eyes;
mod blink_eyes;
mod generate_facets;
mod generate_hexagons;
mod generate_marble;
mod generate_mesh;
mod generate_noise;
mod generate_radial;
mod generate_scales;
mod generate_skulls;
mod generate_textures;
mod palettes;

mod construct_text_overlay;
mod display_overlay;
mod fps_monitor;

mod postprocess_shader;
mod render_passthrough;
mod render_fxaa;
mod render_dithering;
mod render_bloom;
mod select_render_precision;

mod open_database;

// -------------------------------------------------------------------------
// Library-local compatibility shims
// -------------------------------------------------------------------------
//
// The KDE cdylib intentionally does not own the application tray, database
// creation lifecycle, or wallpaper supervisor. A few shared data structures
// mention those modules in type signatures, however. These narrow shims keep
// those signatures compilable without pulling the corresponding host/runtime
// implementations into the shared library. The normal Screenshaver binary is
// unaffected because main.rs declares the real modules in its own crate root.

mod tray_icon {
    #[derive(Debug, Clone)]
    pub struct TrayStatusControl;
}

mod initialize_database {
    pub fn initialize(
        database_path: &std::path::Path,
    ) -> Result<rusqlite::Connection, String> {
        Err(format!(
            "KDE renderer host requires an existing Screenshaver database at '{}'",
            database_path.display(),
        ))
    }
}

mod manage_wallpaper_runtime {
    #[derive(Clone)]
    pub(crate) struct WallpaperPolicyReload {
        pub(crate) animation_speed_policy:
            crate::load_config::AnimationSpeedPolicy,
        pub(crate) fps_policy:
            crate::load_config::FpsPolicy,
        pub(crate) texture_policy:
            crate::load_config::TexturePolicy,
        pub(crate) postprocess_policy:
            crate::load_config::PostprocessPolicy,
    }
}

mod render_frame_engine;
mod render_lock_screen_kde;
