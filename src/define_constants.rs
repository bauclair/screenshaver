pub const WINDOW_TITLE: &str = "Screenshaver";

pub const GL_MAJOR: u8 = 3;
pub const GL_MINOR: u8 = 3;

// Supported render frame rate limits.
pub const MIN_RENDER_FPS: u32 = 16;
pub const DEFAULT_RENDER_FPS: u32 = 30;
pub const MAX_RENDER_FPS: u32 = 120;


// Supported animation-speed limits.
pub const PREVIEW_SPEED_MIN: f32 = 0.01;
pub const PREVIEW_SPEED_DEFAULT: f32 = 1.0;
pub const PREVIEW_SPEED_MAX: f32 = 10.0;

pub const SCREENSAVER_SPEED_MIN: f32 = 0.01;
pub const SCREENSAVER_SPEED_DEFAULT: f32 = 1.0;
pub const SCREENSAVER_SPEED_MAX: f32 = 10.0;

pub const WALLPAPER_SPEED_MIN: f32 = 0.01;
pub const WALLPAPER_SPEED_DEFAULT: f32 = 0.025;
pub const WALLPAPER_SPEED_MAX: f32 = 10.0;


// Supported render-scale limits.
pub const RENDER_SCALE_MIN: f32 = 0.25;
pub const RENDER_SCALE_DEFAULT: f32 = 1.0;
pub const RENDER_SCALE_MAX: f32 = 2.0;

// Supported procedural texture primitive-count limits.
pub const MIN_TEXTURE_PRIMITIVES: usize = 1;
pub const MAX_TEXTURE_PRIMITIVES: usize = 1024;

pub const VERTEX_SHADER: &str = r#"
#version 330 core

void main() {

    vec2 pos = vec2(
        (gl_VertexID << 1) & 2,
        gl_VertexID & 2
    );

    gl_Position = vec4(
        pos * 2.0 - 1.0,
        0.0,
        1.0
    );
}
"#;

