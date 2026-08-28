// Screenshaver KDE renderer shared-library root.
//
// This is intentionally NOT the Screenshaver application module tree. It
// contains only the renderer/policy/configuration dependency graph required
// to construct FrameRenderEngine inside KScreenLocker. Host runtimes such as
// X11/GLX wallpaper, Wayland wallpaper, session management, tray, editors, and
// screen-lock management are deliberately excluded.

#[path = "../../src/load_config.rs"]
mod load_config;
#[path = "../../src/logger.rs"]
mod logger;
#[path = "../../src/parse_mode.rs"]
mod parse_mode;
#[path = "../../src/parse_interval.rs"]
mod parse_interval;
#[path = "../../src/parse_subtitle_placement.rs"]
mod parse_subtitle_placement;
#[path = "../../src/parse_texture_specification.rs"]
mod parse_texture_specification;

#[path = "../../src/define_constants.rs"]
mod define_constants;
#[path = "../../src/define_wallpaper.rs"]
mod define_wallpaper;
#[path = "../../src/locate_paths.rs"]
mod locate_paths;

#[path = "../../src/audio_backend/mod.rs"]
mod audio_backend;
#[path = "../../src/analyze_audio.rs"]
mod analyze_audio;

#[path = "../../src/manage_configuration.rs"]
mod manage_configuration;
#[path = "../../src/manage_shader.rs"]
mod manage_shader;
#[path = "../../src/manage_textures.rs"]
mod manage_textures;
#[path = "../../src/manage_policies.rs"]
mod manage_policies;

#[path = "../../src/classify_shader.rs"]
mod classify_shader;
#[path = "../../src/isf_types.rs"]
mod isf_types;
#[path = "../../src/parse_isf.rs"]
mod parse_isf;
#[path = "../../src/preprocess_isf.rs"]
mod preprocess_isf;
#[path = "../../src/apply_shader_inputs.rs"]
mod apply_shader_inputs;
#[path = "../../src/preprocess_shader.rs"]
mod preprocess_shader;
#[path = "../../src/load_shader.rs"]
mod load_shader;
#[path = "../../src/load_shader_source.rs"]
mod load_shader_source;
#[path = "../../src/compile_shader.rs"]
mod compile_shader;

#[path = "../../src/generate_bricks.rs"]
mod generate_bricks;
#[path = "../../src/generate_cellular.rs"]
mod generate_cellular;
#[path = "../../src/generate_clouds.rs"]
mod generate_clouds;
#[path = "../../src/generate_eyes.rs"]
mod generate_eyes;
#[path = "../../src/blink_eyes.rs"]
mod blink_eyes;
#[path = "../../src/generate_facets.rs"]
mod generate_facets;
#[path = "../../src/generate_hexagons.rs"]
mod generate_hexagons;
#[path = "../../src/generate_marble.rs"]
mod generate_marble;
#[path = "../../src/generate_mesh.rs"]
mod generate_mesh;
#[path = "../../src/generate_noise.rs"]
mod generate_noise;
#[path = "../../src/generate_radial.rs"]
mod generate_radial;
#[path = "../../src/generate_scales.rs"]
mod generate_scales;
#[path = "../../src/generate_skulls.rs"]
mod generate_skulls;
#[path = "../../src/generate_textures.rs"]
mod generate_textures;
#[path = "../../src/palettes.rs"]
mod palettes;

#[path = "../../src/construct_text_overlay.rs"]
mod construct_text_overlay;
#[path = "../../src/display_overlay.rs"]
mod display_overlay;
#[path = "../../src/fps_monitor.rs"]
mod fps_monitor;

#[path = "../../src/postprocess_shader.rs"]
mod postprocess_shader;
#[path = "../../src/render_passthrough.rs"]
mod render_passthrough;
#[path = "../../src/render_fxaa.rs"]
mod render_fxaa;
#[path = "../../src/render_dithering.rs"]
mod render_dithering;
#[path = "../../src/render_bloom.rs"]
mod render_bloom;
#[path = "../../src/select_render_precision.rs"]
mod select_render_precision;

#[path = "../../src/open_database.rs"]
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

#[path = "../../src/render_frame_engine.rs"]
mod render_frame_engine;
#[path = "../../src/render_lock_screen_kde.rs"]
mod render_lock_screen_kde;
