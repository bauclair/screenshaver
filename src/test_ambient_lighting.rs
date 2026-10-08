//! Software-only ambient-lighting proof of concept.
//! Loopback-only OpenRGB SDK v1 simulator; never connects to real hardware or servers.
//! Supports protocol negotiation, one virtual controller, and LED updates.
use std::time::{Duration, Instant};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
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
    println!("[AMBIENT LIGHTING TEST] Loopback OpenRGB SDK protocol v1 simulator (virtual device only).");
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
    let (mut mock_client, mock_state, mock_thread) = start_mock_server()?;
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
            let mut frame = [0u8; KEY_ROWS * KEY_COLS * 3];
            for (index, led) in leds.iter().enumerate() {
                for channel in 0..3 {
                    frame[index * 3 + channel] = float_to_u8(led.rgb[channel]);
                }
            }
            let mut payload = Vec::with_capacity(6 + frame.len() / 3 * 4);
            payload.extend_from_slice(&((6 + KEY_ROWS * KEY_COLS * 4) as u32).to_le_bytes());
            payload.extend_from_slice(&((KEY_ROWS * KEY_COLS) as u16).to_le_bytes());
            for color in frame.chunks_exact(3) {
                payload.extend_from_slice(&[color[0], color[1], color[2], 0]);
            }
            send_packet(&mut mock_client, 0, 1050, &payload)?;
            first_sample = false;
            last_sample = Instant::now();
            sample_count += 1;
        }
        // Display ONLY frames received by the mock TCP server, not the client buffer.
        let displayed = *mock_state.lock()
            .map_err(|_| "Mock lighting state mutex poisoned".to_string())?;
        // GL scissor rectangles are a virtual keyboard, not hardware output.
        let key_width = (WINDOW_WIDTH as i32 - 40) / KEY_COLS as i32;
        let key_height = 35;
        unsafe { gl::Enable(gl::SCISSOR_TEST); }
        for row in 0..KEY_ROWS {
            for col in 0..KEY_COLS {
                let index = row * KEY_COLS + col;
                let led = VirtualLed { rgb: [
                    displayed[index * 3] as f32 / 255.0,
                    displayed[index * 3 + 1] as f32 / 255.0,
                    displayed[index * 3 + 2] as f32 / 255.0,
                ] };
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
    drop(mock_client);
    mock_thread.join()
        .map_err(|_| "Mock lighting server thread panicked".to_string())??;
    println!("[AMBIENT LIGHTING TEST] Completed {sample_count} software samples via loopback TCP; zero hardware writes.");
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



// OpenRGB SDK protocol version 1 is deliberately chosen to keep discovery
// metadata simple while exercising the documented OpenRGB wire format.
// This listener binds to an ephemeral loopback port, never the OpenRGB port 6742.
const SDK_VERSION: u32 = 1;
const LED_COUNT: usize = KEY_ROWS * KEY_COLS;
const MAX_PACKET: usize = 65536;

fn send_packet(socket: &mut TcpStream, device: u32, command: u32, payload: &[u8]) -> Result<(), String> {
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(b"ORGB");
    header[4..8].copy_from_slice(&device.to_le_bytes());
    header[8..12].copy_from_slice(&command.to_le_bytes());
    header[12..16].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    socket.write_all(&header).and_then(|_| socket.write_all(payload))
        .map_err(|e| format!("OpenRGB mock send failed: {e}"))
}

fn recv_packet(socket: &mut TcpStream) -> Result<Option<(u32, u32, Vec<u8>)>, String> {
    let mut header = [0u8; 16];
    match socket.read_exact(&mut header) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof ||
            e.kind() == std::io::ErrorKind::ConnectionReset => return Ok(None),
        Err(e) => return Err(format!("OpenRGB mock header receive failed: {e}")),
    }
    if &header[0..4] != b"ORGB" { return Err("Invalid OpenRGB magic".into()); }
    let device = u32::from_le_bytes(header[4..8].try_into().unwrap());
    let command = u32::from_le_bytes(header[8..12].try_into().unwrap());
    let size = u32::from_le_bytes(header[12..16].try_into().unwrap()) as usize;
    if size > MAX_PACKET { return Err("Oversized OpenRGB packet".into()); }
    let mut payload = vec![0u8; size];
    socket.read_exact(&mut payload).map_err(|e| format!("OpenRGB mock payload receive failed: {e}"))?;
    Ok(Some((device, command, payload)))
}

fn append_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&((value.len() + 1) as u16).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
    out.push(0);
}

fn controller_description() -> Vec<u8> {
    let mut data = vec![0u8; 4]; // data_size includes this field
    data.extend_from_slice(&5i32.to_le_bytes()); // DEVICE_TYPE_KEYBOARD
    append_string(&mut data, "Screenshaver Virtual 70-Key Keyboard");
    append_string(&mut data, "Screenshaver Simulation"); // v1 vendor
    append_string(&mut data, "Loopback-only virtual OpenRGB controller");
    append_string(&mut data, "1.0");
    append_string(&mut data, ""); // serial
    append_string(&mut data, "virtual://screenshaver/keyboard");
    data.extend_from_slice(&0u16.to_le_bytes()); // modes
    data.extend_from_slice(&(-1i32).to_le_bytes()); // active mode
    data.extend_from_slice(&1u16.to_le_bytes()); // zones
    append_string(&mut data, "Virtual Keyboard");
    data.extend_from_slice(&2i32.to_le_bytes()); // ZONE_TYPE_MATRIX
    data.extend_from_slice(&(LED_COUNT as u32).to_le_bytes()); // min
    data.extend_from_slice(&(LED_COUNT as u32).to_le_bytes()); // max
    data.extend_from_slice(&(LED_COUNT as u32).to_le_bytes()); // count
    data.extend_from_slice(&0u16.to_le_bytes()); // no matrix map
    data.extend_from_slice(&(LED_COUNT as u16).to_le_bytes());
    for index in 0..LED_COUNT {
        append_string(&mut data, &format!("Key {:02}", index + 1));
        data.extend_from_slice(&(index as u32).to_le_bytes());
    }
    data.extend_from_slice(&(LED_COUNT as u16).to_le_bytes());
    data.resize(data.len() + LED_COUNT * 4, 0); // initial RGBColor values
    let size = data.len() as u32;
    data[0..4].copy_from_slice(&size.to_le_bytes());
    data
}

fn start_mock_server() -> Result<(
    TcpStream,
    Arc<Mutex<[u8; LED_COUNT * 3]>>,
    thread::JoinHandle<Result<(), String>>,
), String> {
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
        .map_err(|e| format!("Unable to bind loopback OpenRGB simulator: {e}"))?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    let state = Arc::new(Mutex::new([0u8; LED_COUNT * 3]));
    let server_state = Arc::clone(&state);
    let server = thread::spawn(move || -> Result<(), String> {
        let (mut socket, peer) = listener.accept().map_err(|e| e.to_string())?;
        if !peer.ip().is_loopback() { return Err("Rejected non-loopback peer".into()); }
        while let Some((device, command, payload)) = recv_packet(&mut socket)? {
            match command {
                40 if payload.len() == 4 => {
                    send_packet(&mut socket, 0, 40, &SDK_VERSION.to_le_bytes())?;
                }
                0 if payload.is_empty() => {
                    send_packet(&mut socket, 0, 0, &1u32.to_le_bytes())?;
                }
                1 if device == 0 && payload.len() == 4 &&
                    u32::from_le_bytes(payload[..4].try_into().unwrap()) == SDK_VERSION => {
                    send_packet(&mut socket, 0, 1, &controller_description())?;
                }
                50 if payload.last() == Some(&0) => {} // client name
                1050 if device == 0 && payload.len() == 6 + LED_COUNT * 4 => {
                    let declared_size = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize;
                    let count = u16::from_le_bytes(payload[4..6].try_into().unwrap()) as usize;
                    if declared_size != payload.len() || count != LED_COUNT {
                        return Err("Invalid OpenRGB LED update length".into());
                    }
                    let mut frame = [0u8; LED_COUNT * 3];
                    for (index, rgba) in payload[6..].chunks_exact(4).enumerate() {
                        frame[index * 3..index * 3 + 3].copy_from_slice(&rgba[..3]);
                    }
                    *server_state.lock().map_err(|_| "Mock mutex poisoned".to_string())? = frame;
                }
                _ => return Err(format!("Unsupported OpenRGB simulator packet: device={device}, command={command}, length={}", payload.len())),
            }
        }
        Ok(())
    });
    let mut client = TcpStream::connect(address)
        .map_err(|e| format!("Mock client connect failed: {e}"))?;
    client.set_nodelay(true).map_err(|e| e.to_string())?;
    send_packet(&mut client, 0, 40, &SDK_VERSION.to_le_bytes())?;
    let (_, command, reply) = recv_packet(&mut client)?.ok_or("Missing OpenRGB version response")?;
    if command != 40 || reply != SDK_VERSION.to_le_bytes() {
        return Err("OpenRGB simulator protocol version mismatch".into());
    }
    send_packet(&mut client, 0, 50, b"Screenshaver Ambient Test\0")?;
    send_packet(&mut client, 0, 0, &[])?;
    let (_, command, reply) = recv_packet(&mut client)?.ok_or("Missing OpenRGB controller count")?;
    if command != 0 || reply != 1u32.to_le_bytes() {
        return Err("OpenRGB simulator controller count mismatch".into());
    }
    send_packet(&mut client, 0, 1, &SDK_VERSION.to_le_bytes())?;
    let (_, command, reply) = recv_packet(&mut client)?.ok_or("Missing OpenRGB controller data")?;
    if command != 1 || reply.len() < 12 ||
        u32::from_le_bytes(reply[0..4].try_into().unwrap()) as usize != reply.len() {
        return Err("OpenRGB simulator controller description invalid".into());
    }
    println!("[AMBIENT LIGHTING TEST] OpenRGB SDK v1 negotiated; one virtual 70-LED keyboard discovered on ephemeral localhost port.");
    Ok((client, state, server))
}
