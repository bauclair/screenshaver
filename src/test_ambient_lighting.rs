//! Software-only ambient-lighting proof of concept.
//! No HID, USB, OpenRGB, Hyperion, network, or hardware-control operations.
use std::time::{Duration, Instant};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 660;
const SHADER_DISPLAY_HEIGHT: i32 = 420;
const SAMPLE_WIDTH: usize = 32;
const SAMPLE_HEIGHT: usize = 18;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const TEST_DURATION: Duration = Duration::from_secs(60);
const SMOOTHING_ALPHA: f32 = 0.22;
const DARK_PIXEL_THRESHOLD: f32 = 0.055;
const KEY_ROWS: usize = 5;
const KEY_COLS: usize = 14;

const TEST_FRAGMENT_SHADER: &str = r#"
#version 330 core

uniform float iTime;
uniform vec3 iResolution;

out vec4 FragColor;

vec3 palette(float t)
{
    vec3 a = vec3(0.50, 0.50, 0.50);
    vec3 b = vec3(0.50, 0.50, 0.50);
    vec3 c = vec3(1.00, 1.00, 1.00);
    vec3 d = vec3(0.00, 0.33, 0.67);

    return a + b * cos(6.2831853 * (c * t + d));
}

void main()
{
    vec2 uv =
        (2.0 * gl_FragCoord.xy - iResolution.xy)
        / iResolution.y;

    float r = length(uv);
    float angle = atan(uv.y, uv.x);

    float wave =
        sin(3.0 * angle - 2.2 * iTime + 7.0 * r);

    float glow =
        exp(-2.4 * abs(r - (0.48 + 0.12 * wave)));

    float core =
        exp(-3.2 * r);

    float color_phase =
        0.08 * iTime
        + 0.10 * wave
        + 0.12 * r;

    vec3 color =
        palette(color_phase)
        * (0.20 + 1.25 * glow + 0.55 * core);

    color +=
        0.10
        * palette(color_phase + 0.22)
        * (0.5 + 0.5 * sin(4.0 * r - iTime));

    FragColor =
        vec4(
            max(color, vec3(0.0)),
            1.0
        );
}
"#;



/// A virtual LED is a software color only; it has no device handle.
#[derive(Clone, Copy)]
struct VirtualLed { rgb: [f32; 3] }

pub fn run() -> Result<(), String> {
    println!("[AMBIENT LIGHTING TEST] Software-only virtual keyboard; no hardware access.");
    println!("[AMBIENT LIGHTING TEST] Top: GLSL shader. Bottom: virtual 70-key RGB keyboard.");
    println!("[AMBIENT LIGHTING TEST] Press S for spatial mapping, A for soft ambiance, Esc to exit.");
    let sdl = sdl2::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    {
        let attrs = video.gl_attr();
        attrs.set_context_profile(GLProfile::Core);
        attrs.set_context_version(crate::define_constants::GL_MAJOR, crate::define_constants::GL_MINOR);
    }
    let window = video.window("Screenshaver - Virtual Ambient Lighting (S: Spatial / A: Soft)", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered().opengl().build().map_err(|e| e.to_string())?;
    let _context = window.gl_create_context().map_err(|e| e.to_string())?;
    gl::load_with(|name| video.gl_get_proc_address(name) as *const _);
    let _ = video.gl_set_swap_interval(1);
    let program = crate::compile_shader::build_program(crate::define_constants::VERTEX_SHADER, TEST_FRAGMENT_SHADER)
        .map_err(|e| format!("Test shader compilation failed: {e}"))?;
    let mut vao = 0;
    let mut fbo = 0;
    let mut texture = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB8 as i32, SAMPLE_WIDTH as i32, SAMPLE_HEIGHT as i32,
            0, gl::RGB, gl::UNSIGNED_BYTE, std::ptr::null());
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, texture, 0);
        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::DeleteTextures(1, &texture);
            gl::DeleteFramebuffers(1, &fbo);
            gl::DeleteVertexArrays(1, &vao);
            gl::DeleteProgram(program);
            return Err("Virtual ambient-lighting framebuffer is incomplete".to_owned());
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
    let time_uniform = uniform_location(program, b"iTime\0");
    let resolution_uniform = uniform_location(program, b"iResolution\0");
    let mut events = sdl.event_pump().map_err(|e| e.to_string())?;
    let mut pixels = vec![0u8; SAMPLE_WIDTH * SAMPLE_HEIGHT * 3];
    let mut leds = [VirtualLed { rgb: [0.0; 3] }; KEY_ROWS * KEY_COLS];
    let mut soft = [0.0f32; 3];
    let mut spatial = true;
    let mut first_sample = true;
    let start = Instant::now();
    let mut last_sample = start.checked_sub(SAMPLE_INTERVAL).unwrap_or(start);
    let mut sample_count = 0usize;
    'running: while start.elapsed() < TEST_DURATION {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                Event::KeyDown { keycode: Some(Keycode::S), .. } => { spatial = true; println!("[AMBIENT LIGHTING TEST] Spatial mapping"); },
                Event::KeyDown { keycode: Some(Keycode::A), .. } => { spatial = false; println!("[AMBIENT LIGHTING TEST] Soft ambiance"); },
                _ => {}
            }
        }
        let elapsed = start.elapsed().as_secs_f32();
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::Viewport(0, 0, SAMPLE_WIDTH as i32, SAMPLE_HEIGHT as i32);
            gl::UseProgram(program);
            if time_uniform >= 0 { gl::Uniform1f(time_uniform, elapsed); }
            if resolution_uniform >= 0 { gl::Uniform3f(resolution_uniform, SAMPLE_WIDTH as f32, SAMPLE_HEIGHT as f32, 1.0); }
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindFramebuffer(gl::READ_FRAMEBUFFER, fbo);
            gl::BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
            gl::Viewport(0, 0, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32);
            gl::Disable(gl::SCISSOR_TEST);
            gl::ClearColor(0.025, 0.03, 0.045, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::BlitFramebuffer(0, 0, SAMPLE_WIDTH as i32, SAMPLE_HEIGHT as i32,
                0, WINDOW_HEIGHT as i32 - SHADER_DISPLAY_HEIGHT, WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32,
                gl::COLOR_BUFFER_BIT, gl::LINEAR);
        }
        if last_sample.elapsed() >= SAMPLE_INTERVAL {
            unsafe {
                gl::BindFramebuffer(gl::READ_FRAMEBUFFER, fbo);
                gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
                gl::ReadPixels(0, 0, SAMPLE_WIDTH as i32, SAMPLE_HEIGHT as i32,
                    gl::RGB, gl::UNSIGNED_BYTE, pixels.as_mut_ptr() as *mut _);
            }
            let target = representative_color(&pixels);
            for channel in 0..3 {
                soft[channel] = if first_sample { target[channel] }
                    else { soft[channel] + SMOOTHING_ALPHA * (target[channel] - soft[channel]) };
            }
            for row in 0..KEY_ROWS {
                for col in 0..KEY_COLS {
                    let index = row * KEY_COLS + col;
                    let target_rgb = if spatial {
                        // GL readback origin is bottom-left; keyboard rows start at top.
                        let sx = ((col as f32 + 0.5) * SAMPLE_WIDTH as f32 / KEY_COLS as f32) as usize;
                        let sy = ((KEY_ROWS as f32 - row as f32 - 0.5) * SAMPLE_HEIGHT as f32 / KEY_ROWS as f32) as usize;
                        let offset = (sy.min(SAMPLE_HEIGHT - 1) * SAMPLE_WIDTH + sx.min(SAMPLE_WIDTH - 1)) * 3;
                        [pixels[offset] as f32 / 255.0, pixels[offset+1] as f32 / 255.0, pixels[offset+2] as f32 / 255.0]
                    } else { soft };
                    for channel in 0..3 {
                        let old = leds[index].rgb[channel];
                        leds[index].rgb[channel] = if first_sample { target_rgb[channel] }
                            else { old + SMOOTHING_ALPHA * (target_rgb[channel] - old) };
                    }
                }
            }
            first_sample = false;
            last_sample = Instant::now();
            sample_count += 1;
        }
        // GL scissor rectangles are a virtual keyboard, not hardware output.
        let key_width = (WINDOW_WIDTH as i32 - 40) / KEY_COLS as i32;
        let key_height = 35;
        unsafe { gl::Enable(gl::SCISSOR_TEST); }
        for row in 0..KEY_ROWS {
            for col in 0..KEY_COLS {
                let led = leds[row * KEY_COLS + col];
                let x = 20 + col as i32 * key_width + 2;
                let y = 20 + (KEY_ROWS - row - 1) as i32 * 41;
                unsafe {
                    gl::Scissor(x, y, key_width - 5, key_height);
                    gl::ClearColor(led.rgb[0], led.rgb[1], led.rgb[2], 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT);
                }
            }
        }
        unsafe { gl::Disable(gl::SCISSOR_TEST); }
        window.gl_swap_window();
        std::thread::sleep(Duration::from_millis(2));
    }
    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::DeleteTextures(1, &texture);
        gl::DeleteFramebuffers(1, &fbo);
        gl::DeleteVertexArrays(1, &vao);
        gl::DeleteProgram(program);
    }
    println!("[AMBIENT LIGHTING TEST] Completed {sample_count} software samples; zero hardware writes.");
    Ok(())
}

fn representative_color(
    pixels: &[u8],
) -> [f32; 3] {

    let mut weighted =
        [0.0f32; 3];

    let mut total_weight =
        0.0f32;

    let mut fallback =
        [0.0f32; 3];

    let mut fallback_count =
        0.0f32;

    for pixel in
        pixels.chunks_exact(3)
    {
        let red =
            pixel[0] as f32
            / 255.0;

        let green =
            pixel[1] as f32
            / 255.0;

        let blue =
            pixel[2] as f32
            / 255.0;

        fallback[0] += red;
        fallback[1] += green;
        fallback[2] += blue;
        fallback_count += 1.0;

        let luminance =
            0.2126 * red
            + 0.7152 * green
            + 0.0722 * blue;

        if luminance
            < DARK_PIXEL_THRESHOLD
        {
            continue;
        }

        let maximum =
            red.max(
                green.max(
                    blue
                )
            );

        let minimum =
            red.min(
                green.min(
                    blue
                )
            );

        let saturation =
            if maximum > 0.0001 {
                (
                    maximum
                    - minimum
                )
                / maximum
            } else {
                0.0
            };

        // Bright visible pixels matter, but saturated pixels get additional
        // influence so a small vivid feature is not erased by a large dark
        // background.  The floor keeps neutral/white scenes meaningful.
        let weight =
            luminance.sqrt()
            * (
                0.35
                + 0.65 * saturation
            );

        weighted[0] +=
            red * weight;

        weighted[1] +=
            green * weight;

        weighted[2] +=
            blue * weight;

        total_weight +=
            weight;
    }

    if total_weight
        > 0.0001
    {
        [
            weighted[0]
                / total_weight,
            weighted[1]
                / total_weight,
            weighted[2]
                / total_weight,
        ]
    } else if fallback_count
        > 0.0
    {
        [
            fallback[0]
                / fallback_count,
            fallback[1]
                / fallback_count,
            fallback[2]
                / fallback_count,
        ]
    } else {
        [
            0.0,
            0.0,
            0.0,
        ]
    }
}


fn float_to_u8(
    value: f32,
) -> u8 {

    (
        value
            .clamp(
                0.0,
                1.0,
            )
        * 255.0
        + 0.5
    ) as u8
}


fn uniform_location(
    program: u32,
    name: &[u8],
) -> i32 {

    unsafe {
        gl::GetUniformLocation(
            program,
            name.as_ptr()
                as *const i8,
        )
    }
}


