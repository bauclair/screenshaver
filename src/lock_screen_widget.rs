//! lock_screen_widget.rs
//!
//! Circular password-input widget for the Screenshaver lock screen.
//!
//! The widget intentionally displays no text and no persistent indication of
//! password length. Twelve child circles are arranged at 30-degree intervals
//! around a parent radius. The child associated with the current accepted key
//! is highlighted only while that key remains pressed. Authentication failure
//! temporarily turns all twelve child circles red.

use std::ffi::CString;
use std::time::{
    Duration,
    Instant,
    SystemTime,
    UNIX_EPOCH,
};


//
// ------------------------------------------------------------
// Design parameters
// ------------------------------------------------------------
//

use crate::define_lock_screen_widget::LockScreenWidgetConfig;

const CHILD_COUNT: usize = 12;
const CIRCLE_SEGMENTS: usize = 64;


//
// ------------------------------------------------------------
// Widget state
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct LockScreenWidget {
    next_child: usize,
    active_child: Option<usize>,
    fade_started: [Option<Instant>; CHILD_COUNT],
    error_until: Option<Instant>,

    random_state: u64,
    last_random_child: Option<usize>,

    config: LockScreenWidgetConfig,
}


impl LockScreenWidget {
    pub fn new() -> Self {
        Self::with_config(
            LockScreenWidgetConfig::default()
        )
    }


    pub fn with_config(
        config: LockScreenWidgetConfig,
    ) -> Self {
        Self {
            next_child: 0,
            active_child: None,
            fade_started: [None; CHILD_COUNT],
            error_until: None,

            random_state:
                initial_random_state(),

            last_random_child:
                None,

            config,
        }
    }


    /// Record one accepted password-producing key press.
    ///
    /// The currently selected child becomes active and the next accepted key
    /// advances to the following child. After child 11 the sequence wraps to
    /// child 0, so the widget can represent arbitrarily long passwords without
    /// adding new visual elements.
    pub fn key_pressed(
        &mut self,
    ) {
        if self.error_is_active() {
            return;
        }


        let child_index =
            if self.config.randomize_child_display {
                self.next_random_child()
            } else {
                let child_index =
                    self.next_child;

                self.next_child =
                    (
                        self.next_child
                            + 1
                    )
                        % CHILD_COUNT;

                child_index
            };


        self.active_child =
            Some(
                child_index
            );

        self.fade_started[child_index] =
            None;
    }


    fn next_random_child(
        &mut self,
    ) -> usize {
        let excluded =
            self.last_random_child;

        let candidate_count =
            if excluded.is_some() {
                CHILD_COUNT - 1
            } else {
                CHILD_COUNT
            };


        self.random_state =
            xorshift64(
                self.random_state
            );


        let mut child_index =
            (
                self.random_state
                    % candidate_count as u64
            ) as usize;


        if let Some(previous_child) =
            excluded
        {
            if child_index >= previous_child {
                child_index += 1;
            }
        }


        self.last_random_child =
            Some(
                child_index
            );

        child_index
    }


    /// Begin fading the currently active child back to ChildInactiveColor
    /// when the physical key is released.
    pub fn key_released(
        &mut self,
    ) {
        if let Some(child_index) =
            self.active_child.take()
        {
            self.fade_started[child_index] =
                Some(
                    Instant::now()
                );
        }
    }


    /// Keep the child sequence synchronized when Backspace removes a password
    /// character.
    pub fn backspace(
        &mut self,
    ) {
        if self.error_is_active() {
            return;
        }


        self.active_child =
            None;

        self.fade_started =
            [None; CHILD_COUNT];

        self.next_child =
            if self.next_child == 0 {
                CHILD_COUNT - 1
            } else {
                self.next_child - 1
            };
    }


    /// Reset the widget for a new authentication interaction.
    pub fn clear(
        &mut self,
    ) {
        self.next_child =
            0;

        self.active_child =
            None;

        self.fade_started =
            [None; CHILD_COUNT];

        self.error_until =
            None;

        self.last_random_child =
            None;
    }


    /// Display authentication failure without text.
    ///
    /// All child circles become red for the configured duration, after which
    /// they return automatically to ChildInactiveColor.
    pub fn authentication_failed(
        &mut self,
    ) {
        self.next_child =
            0;

        self.active_child =
            None;

        self.fade_started =
            [None; CHILD_COUNT];

        self.error_until =
            Some(
                Instant::now()
                    + self.config.authentication_failure_duration
            );

        self.last_random_child =
            None;
    }


    pub fn error_is_active(
        &self,
    ) -> bool {
        self.error_until
            .map(
                |deadline| {
                    Instant::now()
                        < deadline
                }
            )
            .unwrap_or(
                false
            )
    }


    fn child_color(
        &self,
        child_index: usize,
    ) -> [f32; 4] {
        if self.error_is_active() {
            return self.config.child_error_color;
        }


        if self.active_child
            == Some(child_index)
        {
            return self.config.child_active_color;
        }


        if let Some(fade_started) =
            self.fade_started[child_index]
        {
            let fade_seconds =
                self.config.child_active_fade_time
                    .as_secs_f32();

            if fade_seconds <= f32::EPSILON {
                return self.config.child_inactive_color;
            }

            let progress =
                (
                    fade_started
                        .elapsed()
                        .as_secs_f32()
                    / fade_seconds
                )
                    .clamp(
                        0.0,
                        1.0,
                    );

            if progress < 1.0 {
                return interpolate_color(
                    self.config.child_active_color,
                    self.config.child_inactive_color,
                    progress,
                );
            }
        }


        self.config.child_inactive_color
    }
}


fn initial_random_state(
) -> u64 {
    let time_seed =
        SystemTime::now()
            .duration_since(
                UNIX_EPOCH
            )
            .map(
                |duration| {
                    duration.as_nanos()
                        as u64
                }
            )
            .unwrap_or(
                0x9E37_79B9_7F4A_7C15
            );


    if time_seed == 0 {
        0x9E37_79B9_7F4A_7C15
    } else {
        time_seed
    }
}


fn xorshift64(
    mut state: u64,
) -> u64 {
    if state == 0 {
        state =
            0x9E37_79B9_7F4A_7C15;
    }


    state ^=
        state << 13;

    state ^=
        state >> 7;

    state ^=
        state << 17;

    state
}


fn interpolate_color(
    from: [f32; 4],
    to: [f32; 4],
    progress: f32,
) -> [f32; 4] {
    let progress =
        progress.clamp(
            0.0,
            1.0,
        );

    [
        from[0] + (to[0] - from[0]) * progress,
        from[1] + (to[1] - from[1]) * progress,
        from[2] + (to[2] - from[2]) * progress,
        from[3] + (to[3] - from[3]) * progress,
    ]
}


impl Default
    for LockScreenWidget
{
    fn default() -> Self {
        Self::new()
    }
}


//
// ------------------------------------------------------------
// OpenGL renderer
// ------------------------------------------------------------
//

const VERTEX_SHADER: &str = r#"
#version 330 core

layout(location = 0) in vec2 a_unit_position;

uniform vec2 u_center_pixels;
uniform float u_radius_pixels;
uniform vec2 u_output_size;

out vec2 v_unit_position;

void main() {
    v_unit_position =
        a_unit_position;
    vec2 pixel_position =
        u_center_pixels
        + a_unit_position
            * u_radius_pixels;

    vec2 ndc =
        vec2(
            pixel_position.x
                / u_output_size.x
                * 2.0
                - 1.0,

            1.0
                - pixel_position.y
                    / u_output_size.y
                    * 2.0
        );

    gl_Position =
        vec4(
            ndc,
            0.0,
            1.0
        );
}
"#;


const FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform vec4 u_color;
uniform int u_halo_mode;
uniform float u_halo_inner_ratio;

in vec2 v_unit_position;

out vec4 frag_color;

void main() {
    if (u_halo_mode != 0) {
        float radial_distance =
            length(v_unit_position);

        if (radial_distance < u_halo_inner_ratio) {
            discard;
        }

        float fade =
            1.0
            - smoothstep(
                u_halo_inner_ratio,
                1.0,
                radial_distance
            );

        frag_color =
            vec4(
                u_color.rgb,
                u_color.a * fade
            );

        return;
    }

    frag_color =
        u_color;
}
"#;


pub struct LockScreenWidgetRenderer {
    program: u32,
    vao: u32,
    vbo: u32,

    center_location: i32,
    radius_location: i32,
    output_size_location: i32,
    color_location: i32,
    halo_mode_location: i32,
    halo_inner_ratio_location: i32,

    vertex_count: i32,
}


impl LockScreenWidgetRenderer {
    pub fn new() -> Result<Self, String> {
        let program =
            build_program()?;


        let vertices =
            build_unit_circle_vertices();


        let mut vao =
            0_u32;

        let mut vbo =
            0_u32;


        unsafe {
            gl::GenVertexArrays(
                1,
                &mut vao,
            );

            gl::GenBuffers(
                1,
                &mut vbo,
            );


            gl::BindVertexArray(
                vao
            );

            gl::BindBuffer(
                gl::ARRAY_BUFFER,
                vbo,
            );

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (
                    vertices.len()
                        * std::mem::size_of::<f32>()
                ) as isize,
                vertices.as_ptr()
                    as *const std::ffi::c_void,
                gl::STATIC_DRAW,
            );


            gl::EnableVertexAttribArray(
                0
            );

            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                (
                    2
                        * std::mem::size_of::<f32>()
                ) as i32,
                std::ptr::null(),
            );


            gl::BindBuffer(
                gl::ARRAY_BUFFER,
                0,
            );

            gl::BindVertexArray(
                0
            );
        }


        Ok(
            Self {
                program,
                vao,
                vbo,

                center_location:
                    uniform_location(
                        program,
                        "u_center_pixels",
                    )?,

                radius_location:
                    uniform_location(
                        program,
                        "u_radius_pixels",
                    )?,

                output_size_location:
                    uniform_location(
                        program,
                        "u_output_size",
                    )?,

                color_location:
                    uniform_location(
                        program,
                        "u_color",
                    )?,

                halo_mode_location:
                    uniform_location(
                        program,
                        "u_halo_mode",
                    )?,

                halo_inner_ratio_location:
                    uniform_location(
                        program,
                        "u_halo_inner_ratio",
                    )?,

                vertex_count:
                    (
                        CIRCLE_SEGMENTS
                            + 2
                    ) as i32,
            }
        )
    }


    /// Draw the widget in the center of the secure lock surface.
    pub fn display_centered(
        &self,
        widget: &LockScreenWidget,
        output_width: u32,
        output_height: u32,
    ) {
        if output_width == 0
            || output_height == 0
        {
            return;
        }


        let center_x =
            output_width as f32
                * 0.5;

        let center_y =
            output_height as f32
                * 0.5;


        unsafe {
            gl::Enable(
                gl::BLEND
            );

            gl::BlendFunc(
                gl::SRC_ALPHA,
                gl::ONE_MINUS_SRC_ALPHA,
            );

            gl::UseProgram(
                self.program
            );

            gl::BindVertexArray(
                self.vao
            );

            gl::Uniform2f(
                self.output_size_location,
                output_width as f32,
                output_height as f32,
            );
        }


        self.draw_halo(
            center_x,
            center_y,
            &widget.config,
        );


        self.draw_circle(
            center_x,
            center_y,
            widget.config.background_radius,
            widget.config.background_color,
        );


        for child_index in
            0..CHILD_COUNT
        {
            // Child 0 is at 12 o'clock. Subsequent children advance clockwise.
            let angle =
                (
                    child_index as f32
                        * 30.0
                        - 90.0
                )
                    .to_radians();


            let child_center_x =
                center_x
                    + widget.config.parent_radius
                        * angle.cos();

            let child_center_y =
                center_y
                    + widget.config.parent_radius
                        * angle.sin();


            self.draw_circle(
                child_center_x,
                child_center_y,
                widget.config.child_radius,
                widget.child_color(
                    child_index
                ),
            );
        }


        unsafe {
            gl::BindVertexArray(
                0
            );

            gl::UseProgram(
                0
            );

            gl::Disable(
                gl::BLEND
            );
        }
    }


    fn draw_halo(
        &self,
        center_x: f32,
        center_y: f32,
        config: &LockScreenWidgetConfig,
    ) {
        let halo_strength =
            config.halo_strength.clamp(
                0.0,
                1.0,
            );

        if halo_strength <= f32::EPSILON
            || config.background_radius <= f32::EPSILON
        {
            return;
        }


        let halo_distance =
            config.background_radius
                * halo_strength;

        let outer_radius =
            config.background_radius
                + halo_distance;

        let inner_ratio =
            config.background_radius
                / outer_radius;


        unsafe {
            gl::Uniform1i(
                self.halo_mode_location,
                1,
            );

            gl::Uniform1f(
                self.halo_inner_ratio_location,
                inner_ratio,
            );

            gl::Uniform2f(
                self.center_location,
                center_x,
                center_y,
            );

            gl::Uniform1f(
                self.radius_location,
                outer_radius,
            );

            gl::Uniform4f(
                self.color_location,
                config.halo_color[0],
                config.halo_color[1],
                config.halo_color[2],
                config.halo_color[3],
            );

            gl::DrawArrays(
                gl::TRIANGLE_FAN,
                0,
                self.vertex_count,
            );
        }
    }


    fn draw_circle(
        &self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        color: [f32; 4],
    ) {
        unsafe {
            gl::Uniform1i(
                self.halo_mode_location,
                0,
            );

            gl::Uniform1f(
                self.halo_inner_ratio_location,
                0.0,
            );

            gl::Uniform2f(
                self.center_location,
                center_x,
                center_y,
            );

            gl::Uniform1f(
                self.radius_location,
                radius,
            );

            gl::Uniform4f(
                self.color_location,
                color[0],
                color[1],
                color[2],
                color[3],
            );

            gl::DrawArrays(
                gl::TRIANGLE_FAN,
                0,
                self.vertex_count,
            );
        }
    }
}


impl Drop
    for LockScreenWidgetRenderer
{
    fn drop(
        &mut self,
    ) {
        unsafe {
            if self.vbo != 0 {
                gl::DeleteBuffers(
                    1,
                    &self.vbo,
                );
            }


            if self.vao != 0 {
                gl::DeleteVertexArrays(
                    1,
                    &self.vao,
                );
            }


            if self.program != 0 {
                gl::DeleteProgram(
                    self.program
                );
            }
        }
    }
}


//
// ------------------------------------------------------------
// Circle geometry
// ------------------------------------------------------------
//

fn build_unit_circle_vertices(
) -> Vec<f32> {
    let mut vertices =
        Vec::with_capacity(
            (
                CIRCLE_SEGMENTS
                    + 2
            ) * 2
        );


    vertices.push(
        0.0
    );

    vertices.push(
        0.0
    );


    for segment in
        0..=CIRCLE_SEGMENTS
    {
        let angle =
            segment as f32
                / CIRCLE_SEGMENTS as f32
                * std::f32::consts::TAU;


        vertices.push(
            angle.cos()
        );

        vertices.push(
            angle.sin()
        );
    }


    vertices
}


//
// ------------------------------------------------------------
// OpenGL shader helpers
// ------------------------------------------------------------
//

fn build_program(
) -> Result<u32, String> {
    let vertex_shader =
        compile_shader(
            gl::VERTEX_SHADER,
            VERTEX_SHADER,
        )?;

    let fragment_shader =
        compile_shader(
            gl::FRAGMENT_SHADER,
            FRAGMENT_SHADER,
        )?;


    let program =
        unsafe {
            gl::CreateProgram()
        };


    unsafe {
        gl::AttachShader(
            program,
            vertex_shader,
        );

        gl::AttachShader(
            program,
            fragment_shader,
        );

        gl::LinkProgram(
            program
        );

        gl::DeleteShader(
            vertex_shader
        );

        gl::DeleteShader(
            fragment_shader
        );
    }


    let mut link_status =
        0_i32;


    unsafe {
        gl::GetProgramiv(
            program,
            gl::LINK_STATUS,
            &mut link_status,
        );
    }


    if link_status
        != gl::TRUE as i32
    {
        let message =
            program_log(
                program
            );

        unsafe {
            gl::DeleteProgram(
                program
            );
        }

        return Err(
            format!(
                "Unable to link lock-screen widget shader: {}",
                message,
            )
        );
    }


    Ok(
        program
    )
}


fn compile_shader(
    shader_type: u32,
    source: &str,
) -> Result<u32, String> {
    let shader =
        unsafe {
            gl::CreateShader(
                shader_type
            )
        };


    let source =
        CString::new(
            source
        )
        .map_err(
            |error| {
                format!(
                    "Lock-screen widget shader contains an interior NUL: {}",
                    error,
                )
            }
        )?;


    unsafe {
        gl::ShaderSource(
            shader,
            1,
            &source.as_ptr(),
            std::ptr::null(),
        );

        gl::CompileShader(
            shader
        );
    }


    let mut compile_status =
        0_i32;


    unsafe {
        gl::GetShaderiv(
            shader,
            gl::COMPILE_STATUS,
            &mut compile_status,
        );
    }


    if compile_status
        != gl::TRUE as i32
    {
        let message =
            shader_log(
                shader
            );

        unsafe {
            gl::DeleteShader(
                shader
            );
        }

        return Err(
            format!(
                "Unable to compile lock-screen widget shader: {}",
                message,
            )
        );
    }


    Ok(
        shader
    )
}


fn uniform_location(
    program: u32,
    name: &str,
) -> Result<i32, String> {
    let c_name =
        CString::new(
            name
        )
        .map_err(
            |error| {
                format!(
                    "Invalid lock-screen widget uniform '{}': {}",
                    name,
                    error,
                )
            }
        )?;


    let location =
        unsafe {
            gl::GetUniformLocation(
                program,
                c_name.as_ptr(),
            )
        };


    if location < 0 {
        return Err(
            format!(
                "Lock-screen widget shader uniform '{}' was not found",
                name,
            )
        );
    }


    Ok(
        location
    )
}


fn shader_log(
    shader: u32,
) -> String {
    let mut length =
        0_i32;


    unsafe {
        gl::GetShaderiv(
            shader,
            gl::INFO_LOG_LENGTH,
            &mut length,
        );
    }


    if length <= 1 {
        return "no shader compiler log was provided"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];

    let mut written =
        0_i32;


    unsafe {
        gl::GetShaderInfoLog(
            shader,
            length,
            &mut written,
            buffer.as_mut_ptr()
                as *mut i8,
        );
    }


    String::from_utf8_lossy(
        &buffer[
            ..written.max(0) as usize
        ]
    )
        .to_string()
}


fn program_log(
    program: u32,
) -> String {
    let mut length =
        0_i32;


    unsafe {
        gl::GetProgramiv(
            program,
            gl::INFO_LOG_LENGTH,
            &mut length,
        );
    }


    if length <= 1 {
        return "no program linker log was provided"
            .to_string();
    }


    let mut buffer =
        vec![
            0_u8;
            length as usize
        ];

    let mut written =
        0_i32;


    unsafe {
        gl::GetProgramInfoLog(
            program,
            length,
            &mut written,
            buffer.as_mut_ptr()
                as *mut i8,
        );
    }


    String::from_utf8_lossy(
        &buffer[
            ..written.max(0) as usize
        ]
    )
        .to_string()
}
