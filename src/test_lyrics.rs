use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Deserialize;
use std::ffi::CString;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;

const LRCLIB_GET_URL: &str = "https://lrclib.net/api/get";
const LRCMUX_GET_URL: &str = "https://api.lrcmux.dev/get";
const POSITION_POLL_INTERVAL: Duration = Duration::from_millis(50);
const METADATA_POLL_INTERVAL: Duration = Duration::from_millis(500);
const PLAYER_SCAN_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    id: i64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxResponse {
    track: LrcmuxTrack,
    meta: LrcmuxMeta,
    lines: Vec<LrcmuxLine>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxTrack {
    title: String,
    artist: String,
    album: String,
    duration: u64,
}

#[derive(Debug, Deserialize)]
struct LrcmuxMeta {
    source: LrcmuxSource,
    level: String,
}

#[derive(Debug, Deserialize)]
struct LrcmuxSource {
    id: String,
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct LrcmuxLine {
    text: String,
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackIdentity {
    title: String,
    artist: String,
    album: String,
    duration_milliseconds: u128,
}

#[derive(Debug, Clone)]
struct TrackInformation {
    identity: TrackIdentity,
    duration: Duration,
}

#[derive(Debug, Clone)]
struct SynchronizedLine {
    timestamp: Duration,
    source_timestamp: String,
    text: String,
}

struct LyricsTexture {
    texture: u32,
    width: u32,
    height: u32,
}

impl Drop for LyricsTexture {
    fn drop(&mut self) {
        unsafe {
            if self.texture != 0 && gl::DeleteTextures::is_loaded() {
                gl::DeleteTextures(1, &self.texture);
            }
        }
    }
}

pub fn run() -> Result<(), String> {
    println!("Screenshaver Lyrics Test");
    println!("========================");
    println!();
    println!("Graphical Lyrics Panel prototype");
    println!("Automatically follows an MPRIS player reporting Playing.");
    println!("Press Esc or close the window to stop.");
    println!();

    run_graphical_test()
}

fn find_playing_player() -> Result<Option<Player>, String> {
    let finder = PlayerFinder::new().map_err(|error| {
        format!(
            "Unable to connect to the session D-Bus for MPRIS discovery: {}",
            error
        )
    })?;

    let players = finder
        .find_all()
        .map_err(|error| format!("Unable to enumerate MPRIS media players: {}", error))?;

    for player in players {
        match player.get_playback_status() {
            Ok(PlaybackStatus::Playing) => return Ok(Some(player)),
            Ok(_) => {}
            Err(error) => eprintln!(
                "[LYRICS TEST] Unable to read playback status from {}: {}",
                player.identity(),
                error
            ),
        }
    }

    Ok(None)
}

fn load_player_track(
    player: &Player,
) -> Result<(TrackInformation, Vec<SynchronizedLine>), String> {
    println!();
    println!("Active MPRIS player: {}", player.identity());
    println!("Bus name:            {}", player.bus_name());
    println!();

    let track = read_track_information(player)?;
    print_track_information(&track);
    println!();
    println!("Querying LRCLIB...");
    println!();

    let lines = match retrieve_synchronized_lines(&track) {
        Ok(lines) => lines,
        Err(error) => {
            eprintln!("[LYRICS TEST] {}", error);
            Vec::new()
        }
    };

    Ok((track, lines))
}

fn run_graphical_test() -> Result<(), String> {
    let sdl = sdl2::init().map_err(|error| format!("SDL initialization failed: {}", error))?;
    let video = sdl
        .video()
        .map_err(|error| format!("SDL video initialization failed: {}", error))?;

    {
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }

    let window = video
        .window("Screenshaver Lyrics Panel Test", 1280, 720)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|error| format!("Unable to create lyrics test window: {}", error))?;

    let _gl_context = window
        .gl_create_context()
        .map_err(|error| format!("Unable to create lyrics test OpenGL context: {}", error))?;

    gl::load_with(|symbol| video.gl_get_proc_address(symbol) as *const _);
    let _ = video.gl_set_swap_interval(1);

    let scene_program = build_program(TEST_VERTEX_SHADER, TEST_FRAGMENT_SHADER)?;
    let overlay_program = build_program(OVERLAY_VERTEX_SHADER, OVERLAY_FRAGMENT_SHADER)?;

    let mut vao = 0_u32;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);
    }

    let scene_time = uniform_location(scene_program, "iTime")?;
    let scene_resolution = uniform_location(scene_program, "iResolution")?;
    let overlay_texture_uniform = uniform_location(overlay_program, "panelTexture")?;
    let overlay_rect_uniform = uniform_location(overlay_program, "panelRect")?;

    let mut event_pump = sdl
        .event_pump()
        .map_err(|error| format!("Unable to create lyrics test event pump: {}", error))?;

    let start_time = Instant::now();
    let mut last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
    let mut last_metadata_poll = Instant::now() - METADATA_POLL_INTERVAL;
    let mut last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
    let mut active_player: Option<Player> = None;
    let mut current_track: Option<TrackInformation> = None;
    let mut synchronized_lines: Vec<SynchronizedLine> = Vec::new();
    let mut last_line_index: Option<usize> = None;
    let mut last_output_size = (0_u32, 0_u32);
    let mut panel_texture: Option<LyricsTexture> = None;
    let mut panel_dirty = true;

    'render: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'render,
                Event::Window { .. } => panel_dirty = true,
                _ => {}
            }
        }

        let (drawable_width, drawable_height) = window.drawable_size();
        if drawable_width == 0 || drawable_height == 0 {
            thread::sleep(Duration::from_millis(10));
            continue;
        }

        if (drawable_width, drawable_height) != last_output_size {
            last_output_size = (drawable_width, drawable_height);
            panel_dirty = true;
        }

        // Retain the current player while it remains Playing. If it pauses,
        // stops, disappears, or becomes unreadable, release it and immediately
        // allow another Playing MPRIS player to take over.
        if let Some(player) = active_player.as_ref() {
            match player.get_playback_status() {
                Ok(PlaybackStatus::Playing) => {}
                Ok(status) => {
                    println!();
                    println!(
                        "Active player {} is now {:?}; looking for another Playing player.",
                        player.identity(),
                        status
                    );
                    active_player = None;
                    current_track = None;
                    synchronized_lines.clear();
                    last_line_index = None;
                    panel_dirty = true;
                    last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
                }
                Err(error) => {
                    eprintln!(
                        "[LYRICS TEST] Active player {} is no longer readable: {}",
                        player.identity(),
                        error
                    );
                    active_player = None;
                    current_track = None;
                    synchronized_lines.clear();
                    last_line_index = None;
                    panel_dirty = true;
                    last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
                }
            }
        }

        if active_player.is_none() && last_player_scan.elapsed() >= PLAYER_SCAN_INTERVAL {
            last_player_scan = Instant::now();

            match find_playing_player() {
                Ok(Some(player)) => {
                    match load_player_track(&player) {
                        Ok((track, lines)) => {
                            current_track = Some(track);
                            synchronized_lines = lines;
                            last_line_index = None;
                            active_player = Some(player);
                            last_metadata_poll = Instant::now();
                            last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
                            panel_dirty = true;
                        }
                        Err(error) => {
                            eprintln!(
                                "[LYRICS TEST] Playing player {} could not be used: {}",
                                player.identity(),
                                error
                            );
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => eprintln!("[LYRICS TEST] MPRIS discovery failed: {}", error),
            }
        }

        if last_metadata_poll.elapsed() >= METADATA_POLL_INTERVAL {
            last_metadata_poll = Instant::now();

            if let (Some(player), Some(track)) =
                (active_player.as_ref(), current_track.as_mut())
            {
                match read_track_information(player) {
                    Ok(observed_track) => {
                        if observed_track.identity != track.identity {
                            println!();
                            println!("Track change detected on {}.", player.identity());
                            println!();
                            print_track_information(&observed_track);
                            println!();
                            println!("Querying LRCLIB...");
                            println!();

                            *track = observed_track;
                            synchronized_lines = match retrieve_synchronized_lines(track) {
                                Ok(lines) => lines,
                                Err(error) => {
                                    eprintln!("[LYRICS TEST] {}", error);
                                    Vec::new()
                                }
                            };

                            last_line_index = None;
                            panel_dirty = true;
                        }
                    }
                    Err(error) => eprintln!(
                        "[LYRICS TEST] Unable to refresh track metadata from {}: {}",
                        player.identity(),
                        error
                    ),
                }
            }
        }

        if last_position_poll.elapsed() >= POSITION_POLL_INTERVAL {
            last_position_poll = Instant::now();

            if let Some(player) = active_player.as_ref() {
                if let Ok(position) = player.get_position() {
                    let active = find_active_line(&synchronized_lines, position);
                    if active != last_line_index {
                        last_line_index = active;
                        panel_dirty = true;
                    }
                }
            }
        }

        if panel_dirty {
            panel_texture = build_panel_texture(
                &synchronized_lines,
                last_line_index,
                drawable_width,
                drawable_height,
            )?;
            panel_dirty = false;
        }

        unsafe {
            gl::Viewport(0, 0, drawable_width as i32, drawable_height as i32);
            gl::Disable(gl::BLEND);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(scene_program);
            if scene_time >= 0 {
                gl::Uniform1f(scene_time, start_time.elapsed().as_secs_f32());
            }
            if scene_resolution >= 0 {
                gl::Uniform2f(
                    scene_resolution,
                    drawable_width as f32,
                    drawable_height as f32,
                );
            }
            gl::BindVertexArray(vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);

            if let Some(panel) = panel_texture.as_ref() {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::UseProgram(overlay_program);
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, panel.texture);
                if overlay_texture_uniform >= 0 {
                    gl::Uniform1i(overlay_texture_uniform, 0);
                }

                let width_ratio = panel.width as f32 / drawable_width as f32;
                let height_ratio = panel.height as f32 / drawable_height as f32;
                if overlay_rect_uniform >= 0 {
                    gl::Uniform4f(
                        overlay_rect_uniform,
                        0.0,
                        0.0,
                        width_ratio,
                        height_ratio,
                    );
                }
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                gl::Disable(gl::BLEND);
            }
        }

        window.gl_swap_window();
    }

    panel_texture = None;
    drop(panel_texture);

    unsafe {
        gl::UseProgram(0);
        gl::BindVertexArray(0);
        gl::DeleteVertexArrays(1, &vao);
        gl::DeleteProgram(scene_program);
        gl::DeleteProgram(overlay_program);
    }

    Ok(())
}

fn build_panel_texture(
    lines: &[SynchronizedLine],
    active: Option<usize>,
    output_width: u32,
    output_height: u32,
) -> Result<Option<LyricsTexture>, String> {
    let Some(active) = active else {
        return Ok(None);
    };

    if lines.is_empty() || active >= lines.len() {
        return Ok(None);
    }

    let previous = active.checked_sub(1).and_then(|index| lines.get(index)).map(|line| line.text.as_str());
    let current = lines.get(active).map(|line| line.text.as_str());
    let next = lines.get(active + 1).map(|line| line.text.as_str());

    let panel = crate::construct_text_overlay::construct_lyrics_panel(
        previous,
        current,
        next,
        output_width,
        output_height,
    )?;

    let mut texture = 0_u32;
    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA8 as i32,
            panel.width as i32,
            panel.height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            panel.pixels.as_ptr() as *const _,
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    Ok(Some(LyricsTexture {
        texture,
        width: panel.width,
        height: panel.height,
    }))
}

fn build_program(vertex_source: &str, fragment_source: &str) -> Result<u32, String> {
    let vertex = compile_gl_shader(gl::VERTEX_SHADER, vertex_source)?;
    let fragment = compile_gl_shader(gl::FRAGMENT_SHADER, fragment_source)?;
    let program = unsafe { gl::CreateProgram() };

    unsafe {
        gl::AttachShader(program, vertex);
        gl::AttachShader(program, fragment);
        gl::LinkProgram(program);
    }

    let mut linked = 0_i32;
    unsafe { gl::GetProgramiv(program, gl::LINK_STATUS, &mut linked) };

    unsafe {
        gl::DeleteShader(vertex);
        gl::DeleteShader(fragment);
    }

    if linked == 0 {
        let error = program_log(program);
        unsafe { gl::DeleteProgram(program) };
        return Err(format!("Lyrics test OpenGL program link failed: {}", error));
    }

    Ok(program)
}

fn compile_gl_shader(kind: u32, source: &str) -> Result<u32, String> {
    let shader = unsafe { gl::CreateShader(kind) };
    let source = CString::new(source).map_err(|_| "Shader source contains an interior NUL byte".to_string())?;
    unsafe {
        let pointer = source.as_ptr();
        gl::ShaderSource(shader, 1, &pointer, ptr::null());
        gl::CompileShader(shader);
    }

    let mut compiled = 0_i32;
    unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut compiled) };
    if compiled == 0 {
        let error = shader_log(shader);
        unsafe { gl::DeleteShader(shader) };
        return Err(format!("Lyrics test OpenGL shader compilation failed: {}", error));
    }
    Ok(shader)
}

fn shader_log(shader: u32) -> String {
    let mut length = 0_i32;
    unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut length) };
    if length <= 1 { return "unknown shader error".to_string(); }
    let mut bytes = vec![0_u8; length as usize];
    unsafe { gl::GetShaderInfoLog(shader, length, ptr::null_mut(), bytes.as_mut_ptr() as *mut i8) };
    String::from_utf8_lossy(&bytes).trim_matches(char::from(0)).to_string()
}

fn program_log(program: u32) -> String {
    let mut length = 0_i32;
    unsafe { gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut length) };
    if length <= 1 { return "unknown program link error".to_string(); }
    let mut bytes = vec![0_u8; length as usize];
    unsafe { gl::GetProgramInfoLog(program, length, ptr::null_mut(), bytes.as_mut_ptr() as *mut i8) };
    String::from_utf8_lossy(&bytes).trim_matches(char::from(0)).to_string()
}

fn uniform_location(program: u32, name: &str) -> Result<i32, String> {
    let name = CString::new(name).map_err(|_| format!("Uniform name contains an interior NUL byte: {}", name))?;
    Ok(unsafe { gl::GetUniformLocation(program, name.as_ptr()) })
}

const TEST_VERTEX_SHADER: &str = r#"#version 330 core
out vec2 uv;
void main() {
    vec2 p;
    if (gl_VertexID == 0) p = vec2(-1.0, -1.0);
    else if (gl_VertexID == 1) p = vec2(3.0, -1.0);
    else p = vec2(-1.0, 3.0);
    uv = p * 0.5 + 0.5;
    gl_Position = vec4(p, 0.0, 1.0);
}
"#;

const TEST_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 uv;
out vec4 fragColor;
uniform float iTime;
uniform vec2 iResolution;

void main() {
    vec2 fragCoord = gl_FragCoord.xy;
    vec2 coord = fragCoord / max(iResolution, vec2(1.0));
    vec2 p = (2.0 * fragCoord - iResolution) / max(iResolution.y, 1.0);

    float t = iTime * 0.35;

    float wave1 = 0.5 + 0.5 * sin(
        p.x * 4.0 +
        p.y * 2.5 +
        t * 2.0
    );

    float wave2 = 0.5 + 0.5 * cos(
        p.x * 2.0 -
        p.y * 5.0 -
        t * 1.4
    );

    float bands = 0.5 + 0.5 * sin(
        (coord.x + coord.y) * 18.0 -
        t * 3.0
    );

    float detail =
        0.5 +
        0.5 * sin(
            coord.x * 80.0 +
            sin(coord.y * 20.0 + t) * 4.0
        );

    vec3 base = vec3(
        0.82 + 0.18 * wave1,
        0.84 + 0.16 * wave2,
        0.80 + 0.20 * bands
    );

    vec3 tint = vec3(
        0.08 * wave2,
        0.07 * bands,
        0.08 * wave1
    );

    vec3 color = base + tint;

    float radius = length(
        p - vec2(
            0.35 * sin(t),
            0.18 * cos(t * 1.3)
        )
    );

    float glow = exp(-radius * radius * 2.5);

    color += vec3(0.20, 0.18, 0.12) * glow;
    color += vec3(0.045 * detail);

    // Deliberately keep every part of the image bright so the lyrics
    // background is tested under hostile, high-luminance conditions.
    color = max(color, vec3(0.80));

    fragColor = vec4(clamp(color, 0.0, 1.0), 1.0);
}
"#;

const OVERLAY_VERTEX_SHADER: &str = r#"#version 330 core
out vec2 texCoord;
uniform vec4 panelRect;
void main() {
    vec2 corners[6] = vec2[6](
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)
    );
    vec2 c = corners[gl_VertexID];

    // panelRect.zw are fractions of the drawable viewport. Anchor the lyrics
    // texture to the bottom-left and convert those fractions to OpenGL's
    // -1..+1 clip-space span.
    vec2 size = panelRect.zw;
    vec2 clip = vec2(
        -1.0 + c.x * size.x * 2.0,
        -1.0 + c.y * size.y * 2.0
    );
    gl_Position = vec4(clip, 0.0, 1.0);
    texCoord = vec2(c.x, 1.0 - c.y);
}
"#;

const OVERLAY_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 texCoord;
out vec4 fragColor;
uniform sampler2D panelTexture;
void main() {
    fragColor = texture(panelTexture, texCoord);
}
"#;

fn read_track_information(
    player: &Player,
) -> Result<TrackInformation, String> {

    let metadata =
        player.get_metadata()
            .map_err(
                |error| {
                    format!(
                        "Unable to obtain metadata from {}: {}",
                        player.identity(),
                        error
                    )
                }
            )?;


    let title =
        metadata.title()
            .ok_or_else(
                || {
                    "The selected MPRIS player did not provide a track title."
                        .to_string()
                }
            )?
            .to_string();


    let artist =
        metadata.artists()
            .and_then(
                |artists| {
                    artists.first()
                        .copied()
                }
            )
            .ok_or_else(
                || {
                    "The selected MPRIS player did not provide an artist."
                        .to_string()
                }
            )?
            .to_string();


    let album =
        metadata.album_name()
            .unwrap_or("")
            .to_string();


    let duration =
        metadata.length()
            .ok_or_else(
                || {
                    "The selected MPRIS player did not provide a track duration."
                        .to_string()
                }
            )?;


    Ok(
        TrackInformation {
            identity: TrackIdentity {
                title,
                artist,
                album,
                duration_milliseconds:
                    duration.as_millis(),
            },
            duration,
        }
    )
}


fn print_track_information(
    track: &TrackInformation,
) {

    println!("Track");
    println!("-----");

    println!(
        "Title:     {}",
        track.identity.title
    );

    println!(
        "Artist:    {}",
        track.identity.artist
    );

    println!(
        "Album:     {}",
        if track.identity.album.is_empty() {
            "<not provided>"
        } else {
            &track.identity.album
        }
    );

    println!(
        "Duration:  {}",
        format_duration(
            track.duration
        )
    );
}


fn retrieve_synchronized_lines(
    track: &TrackInformation,
) -> Result<Vec<SynchronizedLine>, String> {
    match retrieve_lrclib_synchronized_lines(track) {
        Ok(lines) => {
            println!("Lyrics provider:      LRCLIB");
            Ok(lines)
        }
        Err(lrclib_error) => {
            eprintln!("[LYRICS TEST] {}", lrclib_error);
            println!();
            println!("Trying fallback provider: LRCMUX...");
            println!();

            match retrieve_lrcmux_synchronized_lines(track) {
                Ok(lines) => {
                    println!("Lyrics provider:      LRCMUX");
                    Ok(lines)
                }
                Err(lrcmux_error) => Err(format!(
                    "No synchronized lyrics available. LRCLIB: {} LRCMUX: {}",
                    lrclib_error,
                    lrcmux_error
                )),
            }
        }
    }
}

fn retrieve_lrclib_synchronized_lines(
    track: &TrackInformation,
) -> Result<Vec<SynchronizedLine>, String> {
    let result = query_lrclib(
        &track.identity.title,
        &track.identity.artist,
        &track.identity.album,
        track.duration,
    )?;

    println!("LRCLIB match found.");
    println!("LRCLIB ID:           {}", result.id);
    println!("Matched track:       {}", result.track_name);
    println!("Matched artist:      {}", result.artist_name);
    println!(
        "Matched album:       {}",
        if result.album_name.trim().is_empty() {
            "<not provided>"
        } else {
            &result.album_name
        }
    );
    println!("Matched duration:    {:.3} seconds", result.duration);
    println!("Instrumental:        {}", yes_no(result.instrumental));
    println!(
        "Plain lyrics:        {}",
        yes_no(has_text(result.plain_lyrics.as_deref()))
    );
    println!(
        "Synchronized lyrics: {}",
        yes_no(has_text(result.synced_lyrics.as_deref()))
    );

    let synced_lyrics = result
        .synced_lyrics
        .as_deref()
        .filter(|lyrics| has_text(Some(lyrics)))
        .ok_or_else(|| {
            format!(
                "LRCLIB returned no synchronized lyrics for '{}' by '{}'.",
                track.identity.title, track.identity.artist
            )
        })?;

    let synchronized_lines = parse_synchronized_lyrics(synced_lyrics)?;
    println!("Synchronized lines:  {}", synchronized_lines.len());
    Ok(synchronized_lines)
}

fn retrieve_lrcmux_synchronized_lines(
    track: &TrackInformation,
) -> Result<Vec<SynchronizedLine>, String> {
    let result = query_lrcmux(
        &track.identity.title,
        &track.identity.artist,
        &track.identity.album,
        track.duration,
    )?;

    if result.lines.is_empty() {
        return Err(format!(
            "LRCMUX returned no synchronized lyrics for '{}' by '{}'.",
            track.identity.title, track.identity.artist
        ));
    }

    println!("LRCMUX match found.");
    println!("Matched track:       {}", result.track.title);
    println!("Matched artist:      {}", result.track.artist);
    println!(
        "Matched album:       {}",
        if result.track.album.trim().is_empty() {
            "<not provided>"
        } else {
            &result.track.album
        }
    );
    println!("Matched duration:    {} seconds", result.track.duration);
    println!(
        "LRCMUX source:       {} ({})",
        result.meta.source.name, result.meta.source.id
    );
    println!("LRCMUX source URL:   {}", result.meta.source.url);
    println!("Synchronization:     {}", result.meta.level);

    let mut synchronized_lines = Vec::with_capacity(result.lines.len());
    for line in result.lines {
        let timestamp = Duration::from_millis(line.start);
        let source_timestamp = format!("{}-{} ms", line.start, line.end);
        synchronized_lines.push(SynchronizedLine {
            timestamp,
            source_timestamp,
            text: line.text,
        });
    }
    synchronized_lines.sort_by_key(|line| line.timestamp);

    println!("Synchronized lines:  {}", synchronized_lines.len());
    Ok(synchronized_lines)
}


fn parse_synchronized_lyrics(
    lyrics: &str,
) -> Result<Vec<SynchronizedLine>, String> {

    let mut lines =
        Vec::new();


    for raw_line in lyrics.lines() {

        let raw_line =
            raw_line.trim();


        if raw_line.is_empty() {
            continue;
        }


        let Some(
            closing_bracket
        ) =
            raw_line.find(
                ']'
            )
        else {
            continue;
        };


        if !raw_line.starts_with(
            '['
        ) {
            continue;
        }


        let timestamp_text =
            &raw_line[
                1..closing_bracket
            ];


        let Some(timestamp) =
            parse_lrc_timestamp(
                timestamp_text
            )
        else {
            continue;
        };


        let text =
            raw_line[
                closing_bracket + 1..
            ]
                .trim()
                .to_string();


        lines.push(
            SynchronizedLine {
                timestamp,
                source_timestamp:
                    format!(
                        "[{}]",
                        timestamp_text
                    ),
                text,
            }
        );
    }


    if lines.is_empty() {
        return Err(
            "LRCLIB synchronized lyrics contained no parseable timestamped lines."
                .to_string()
        );
    }


    lines.sort_by_key(
        |line| {
            line.timestamp
        }
    );


    Ok(
        lines
    )
}


fn parse_lrc_timestamp(
    timestamp: &str,
) -> Option<Duration> {

    let (
        minutes_text,
        seconds_text,
    ) =
        timestamp.split_once(
            ':'
        )?;


    let minutes =
        minutes_text.parse::<u64>()
            .ok()?;


    let seconds =
        seconds_text.parse::<f64>()
            .ok()?;


    if !seconds.is_finite()
        || seconds < 0.0
        || seconds >= 60.0
    {
        return None;
    }


    let total_seconds =
        (minutes as f64)
            * 60.0
            + seconds;


    Some(
        Duration::from_secs_f64(
            total_seconds
        )
    )
}


fn find_active_line(
    lines: &[SynchronizedLine],
    position: Duration,
) -> Option<usize> {

    if lines.is_empty()
        || position < lines[0].timestamp
    {
        return None;
    }


    match lines.binary_search_by_key(
        &position,
        |line| {
            line.timestamp
        },
    ) {
        Ok(index) => {
            Some(
                index
            )
        }

        Err(0) => {
            None
        }

        Err(index) => {
            Some(
                index - 1
            )
        }
    }
}


fn query_lrclib(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
) -> Result<LrclibResponse, String> {

    const MAX_RETRIES: usize = 3;
    const RETRY_DELAYS_SECONDS: [u64; MAX_RETRIES] = [1, 2, 4];

    let client =
        reqwest::blocking::Client::builder()
            .user_agent(
                concat!(
                    "Screenshaver/",
                    env!("CARGO_PKG_VERSION"),
                    " lyrics-test"
                )
            )
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create the LRCLIB HTTP client: {}",
                        error
                    )
                }
            )?;


    let duration_string =
        format!(
            "{:.3}",
            duration.as_secs_f64()
        );


    let mut query =
        vec![
            (
                "track_name",
                title.to_string(),
            ),
            (
                "artist_name",
                artist.to_string(),
            ),
            (
                "duration",
                duration_string,
            ),
        ];


    if !album.trim().is_empty() {
        query.push(
            (
                "album_name",
                album.to_string(),
            )
        );
    }


    for attempt in 0..=MAX_RETRIES {
        let response =
            client.get(
                LRCLIB_GET_URL
            )
                .query(
                    &query
                )
                .send();


        let response =
            match response {
                Ok(response) => response,

                Err(error) => {
                    if attempt < MAX_RETRIES {
                        let delay_seconds = RETRY_DELAYS_SECONDS[attempt];

                        eprintln!(
                            "[LYRICS TEST] Unable to contact LRCLIB: {}",
                            error
                        );
                        eprintln!(
                            "[LYRICS TEST] Retrying LRCLIB in {} second{}... ({}/{})",
                            delay_seconds,
                            if delay_seconds == 1 { "" } else { "s" },
                            attempt + 1,
                            MAX_RETRIES
                        );

                        thread::sleep(
                            Duration::from_secs(delay_seconds)
                        );
                        continue;
                    }

                    return Err(
                        format!(
                            "Unable to contact LRCLIB after {} attempts: {}",
                            MAX_RETRIES + 1,
                            error
                        )
                    );
                }
            };


        let status =
            response.status();


        if status
            == reqwest::StatusCode::NOT_FOUND
        {
            return Err(
                format!(
                    "LRCLIB did not find a matching track for '{}' by '{}'.",
                    title,
                    artist
                )
            );
        }


        let retryable_status =
            status.is_server_error()
                || status == reqwest::StatusCode::TOO_MANY_REQUESTS;


        if retryable_status && attempt < MAX_RETRIES {
            let retry_after_seconds =
                response.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.trim().parse::<u64>().ok());

            let delay_seconds =
                retry_after_seconds
                    .unwrap_or(RETRY_DELAYS_SECONDS[attempt]);

            eprintln!(
                "[LYRICS TEST] LRCLIB returned HTTP status {}.",
                status
            );
            eprintln!(
                "[LYRICS TEST] Retrying LRCLIB in {} second{}... ({}/{})",
                delay_seconds,
                if delay_seconds == 1 { "" } else { "s" },
                attempt + 1,
                MAX_RETRIES
            );

            thread::sleep(
                Duration::from_secs(delay_seconds)
            );
            continue;
        }


        if !status.is_success() {
            return Err(
                format!(
                    "LRCLIB returned HTTP status {}{}.",
                    status,
                    if retryable_status {
                        format!(
                            " after {} attempts",
                            MAX_RETRIES + 1
                        )
                    } else {
                        String::new()
                    }
                )
            );
        }


        return response.json::<LrclibResponse>()
            .map_err(
                |error| {
                    format!(
                        "Unable to decode the LRCLIB response: {}",
                        error
                    )
                }
            );
    }


    Err(
        "LRCLIB retry loop ended unexpectedly."
            .to_string()
    )
}


fn query_lrcmux(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
) -> Result<LrcmuxResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!(
            "Screenshaver/",
            env!("CARGO_PKG_VERSION"),
            " lyrics-test"
        ))
        .build()
        .map_err(|error| format!("Unable to create the LRCMUX HTTP client: {}", error))?;

    let mut query = vec![
        ("artist", artist.to_string()),
        ("title", title.to_string()),
        ("duration", duration.as_secs().to_string()),
    ];

    if !album.trim().is_empty() {
        query.push(("album", album.to_string()));
    }

    let response = client
        .get(LRCMUX_GET_URL)
        .query(&query)
        .send()
        .map_err(|error| format!("Unable to contact LRCMUX: {}", error))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(format!(
            "LRCMUX did not find a matching track for '{}' by '{}'.",
            title, artist
        ));
    }

    if !status.is_success() {
        return Err(format!("LRCMUX returned HTTP status {}.", status));
    }

    response
        .json::<LrcmuxResponse>()
        .map_err(|error| format!("Unable to decode the LRCMUX response: {}", error))
}


fn has_text(
    value: Option<&str>,
) -> bool {

    value
        .map(
            |text| {
                !text.trim().is_empty()
            }
        )
        .unwrap_or(
            false
        )
}


fn yes_no(
    value: bool,
) -> &'static str {

    if value {
        "Yes"
    } else {
        "No"
    }
}


fn format_duration(
    duration: Duration,
) -> String {

    let total_milliseconds =
        duration.as_millis();


    let minutes =
        total_milliseconds
            / 60_000;


    let seconds =
        (
            total_milliseconds
                / 1_000
        )
            % 60;


    let milliseconds =
        total_milliseconds
            % 1_000;


    format!(
        "{:02}:{:02}.{:03}",
        minutes,
        seconds,
        milliseconds
    )
}
