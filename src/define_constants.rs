pub const WINDOW_TITLE: &str = "Screenshaver";

pub const GL_MAJOR: u8 = 3;
pub const GL_MINOR: u8 = 3;

pub const DEFAULT_RENDER_FPS: u32 = 60;

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