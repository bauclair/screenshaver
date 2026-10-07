use std::fs::{
    self,
    OpenOptions,
};

use std::io;

use std::thread;
use std::time::{
    Duration,
    Instant,
};

use std::os::fd::AsRawFd;

use std::path::{
    Path,
    PathBuf,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;


const TARGET_VENDOR_ID: &str = "3151";
const TARGET_PRODUCT_ID: &str = "5030";

const TARGET_VENDOR_ID_NUMERIC: u16 = 0x3151;
const TARGET_PRODUCT_ID_NUMERIC: u16 = 0x5030;

const TARGET_INTERFACE_NUMBER: u8 = 2;

const FEATURE_REPORT_PAYLOAD_LENGTH: usize = 64;
const FEATURE_REPORT_BUFFER_LENGTH: usize =
    FEATURE_REPORT_PAYLOAD_LENGTH + 1;

const IDENTIFY_OPCODE: u8 = 0x8F;
const SET_LEDPARAM_OPCODE: u8 = 0x07;
const GET_LEDPARAM_OPCODE: u8 = 0x87;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;

const SAMPLE_WIDTH: usize = 32;
const SAMPLE_HEIGHT: usize = 18;

const CONSTANT_PHASE_DURATION: Duration = Duration::from_secs(5);
const SHADER_PHASE_DURATION: Duration = Duration::from_secs(20);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(50);          // 20 Hz
const CONSTANT_OUTPUT_INTERVAL: Duration = Duration::from_millis(200); // 5 Hz
const SHADER_OUTPUT_INTERVAL: Duration = Duration::from_millis(500);   // 2 Hz
const RESTORE_SETTLE_DURATION: Duration = Duration::from_millis(150);

const AMBIENT_BRIGHTNESS: u8 = 2;
const SMOOTHING_ALPHA: f32 = 0.22;
const DARK_PIXEL_THRESHOLD: f32 = 0.055;

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


// Linux hidraw ioctl encoding.  The buffer passed to HIDIOCSFEATURE and
// HIDIOCGFEATURE includes the mandatory HID report-ID byte.  This keyboard
// uses report ID 0, followed by its 64-byte vendor-defined feature report.
const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;

const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 =
    IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 =
    IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 =
    IOC_SIZESHIFT + IOC_SIZEBITS;

const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;

const HIDRAW_IOCTL_TYPE: u32 = b'H' as u32;
const HIDIOCSFEATURE_NR: u32 = 0x06;
const HIDIOCGFEATURE_NR: u32 = 0x07;


unsafe extern "C" {
    fn ioctl(
        fd: i32,
        request: usize,
        ...
    ) -> i32;
}


#[derive(Debug)]
struct CandidateDevice {
    hidraw_path: PathBuf,
    interface_number: u8,
    report_descriptor: Vec<u8>,
}


#[derive(Clone, Debug)]
struct LedParameters {
    mode: u8,
    speed: u8,
    brightness: u8,
    option: u8,
    flags: u8,
    red: u8,
    green: u8,
    blue: u8,
}


pub fn run() -> Result<(), String> {

    println!(
        "[AMBIENT LIGHTING TEST] Screenshaver shader-driven ambient-lighting proof of concept"
    );

    println!(
        "[AMBIENT LIGHTING TEST] Searching for GamaKay TK75-TMR USB {:04X}:{:04X}...",
        TARGET_VENDOR_ID_NUMERIC,
        TARGET_PRODUCT_ID_NUMERIC,
    );

    let candidates =
        discover_target_hidraw_devices()?;

    if candidates.is_empty() {
        return Err(
            format!(
                "No HID interfaces were found for USB {:04X}:{:04X}. Verify that the TK75-TMR is connected by USB.",
                TARGET_VENDOR_ID_NUMERIC,
                TARGET_PRODUCT_ID_NUMERIC,
            )
        );
    }

    println!(
        "[AMBIENT LIGHTING TEST] Found {} matching HID interface(s).",
        candidates.len()
    );

    let candidate =
        candidates
            .into_iter()
            .find(
                |candidate| {
                    candidate.interface_number
                        == TARGET_INTERFACE_NUMBER
                        && is_expected_vendor_feature_descriptor(
                            &candidate.report_descriptor
                        )
                }
            )
            .ok_or_else(
                || {
                    format!(
                        "USB {:04X}:{:04X} is present, but its interface {} does not expose the expected vendor-defined 64-byte Feature Report. No HID command was sent.",
                        TARGET_VENDOR_ID_NUMERIC,
                        TARGET_PRODUCT_ID_NUMERIC,
                        TARGET_INTERFACE_NUMBER,
                    )
                }
            )?;

    println!(
        "[AMBIENT LIGHTING TEST] Candidate: {}",
        candidate.hidraw_path.display()
    );

    println!(
        "[AMBIENT LIGHTING TEST] USB interface: {}",
        candidate.interface_number
    );

    println!(
        "[AMBIENT LIGHTING TEST] Report descriptor matches vendor page 0xFFFF, usage 0x02, 64-byte Feature Report."
    );

    let device_id =
        identify_device(
            &candidate.hidraw_path
        )?;

    println!(
        "[AMBIENT LIGHTING TEST] Device identify response: {} (0x{:08X})",
        device_id,
        device_id,
    );

    let description =
        classify_tk75_tmr_device_id(
            device_id
        )
        .ok_or_else(
            || {
                format!(
                    "USB {:04X}:{:04X} returned device ID {}, which is not in the TK75-TMR allow-list. No lighting-setting command was sent.",
                    TARGET_VENDOR_ID_NUMERIC,
                    TARGET_PRODUCT_ID_NUMERIC,
                    device_id,
                )
            }
        )?;

    println!(
        "[AMBIENT LIGHTING TEST] Recognized TK75-TMR revision: {}",
        description
    );

    println!(
        "[AMBIENT LIGHTING TEST] Protocol family: gen2"
    );

    println!(
        "[AMBIENT LIGHTING TEST] Identity gate passed; reading current LED parameters (0x87)..."
    );

    let original =
        read_led_parameters(
            &candidate.hidraw_path
        )?;

    print_led_parameters(
        "Saved original",
        &original,
    );

    println!(
        "[AMBIENT LIGHTING TEST] Starting controlled shader renderer."
    );

    println!(
        "[AMBIENT LIGHTING TEST] Phase A: resend identical RGB #4060A0 at 5 Hz for 5 seconds."
    );

    println!(
        "[AMBIENT LIGHTING TEST] Phase B: {}x{} shader analysis at 20 Hz; shader-derived hardware output at 2 Hz for 20 seconds.",
        SAMPLE_WIDTH,
        SAMPLE_HEIGHT,
    );

    println!(
        "[AMBIENT LIGHTING TEST] Soft Ambiance algorithm is unchanged: dark-pixel suppression + luminance/saturation weighting + temporal smoothing."
    );

    println!(
        "[AMBIENT LIGHTING TEST] Press Esc or close the window to stop early."
    );

    println!(
        "[AMBIENT LIGHTING TEST] Original keyboard lighting is held in memory and will be restored on exit."
    );

    let shader_result =
        run_shader_ambient_test(
            &candidate.hidraw_path
        );

    println!(
        "[AMBIENT LIGHTING TEST] Restoring the exact saved LED parameters..."
    );

    if let Err(error) =
        set_led_parameters(
            &candidate.hidraw_path,
            &original,
        )
    {
        return Err(
            format!(
                "CRITICAL: shader-driven ambient test ended, but restoration failed: {}. Unplug/reconnect the keyboard before further ambient-lighting tests.",
                error,
            )
        );
    }

    thread::sleep(
        RESTORE_SETTLE_DURATION
    );

    println!(
        "[AMBIENT LIGHTING TEST] Restore write sent; reading LED parameters back for verification..."
    );

    let restored =
        read_led_parameters(
            &candidate.hidraw_path
        )?;

    print_led_parameters(
        "Restored read-back",
        &restored,
    );

    if !led_parameters_equal(
        &original,
        &restored,
    ) {
        return Err(
            "Restoration read-back does not exactly match the saved original LED parameters. No further lighting command was sent.".to_string()
        );
    }

    if let Err(error) = shader_result {
        return Err(
            format!(
                "Shader-driven ambient test stopped with an error, but the original keyboard lighting was restored exactly: {}",
                error,
            )
        );
    }

    println!(
        "[AMBIENT LIGHTING TEST] PASS: constant-color 5 Hz and shader-derived 2 Hz phases completed; the original LED parameters were restored exactly."
    );

    Ok(())
}


fn run_shader_ambient_test(
    hidraw_path: &Path,
) -> Result<(), String> {

    let constant_parameters =
        LedParameters {
            mode: 1,
            speed: 2,
            brightness:
                AMBIENT_BRIGHTNESS,
            option: 0,
            flags: 7,
            red: 0x40,
            green: 0x60,
            blue: 0xA0,
        };

    println!(
        "[AMBIENT LIGHTING TEST] Phase A starting: identical static RGB #4060A0, 5 Hz, 5 seconds."
    );

    let constant_started =
        Instant::now();

    let mut constant_count =
        0usize;

    while constant_started.elapsed()
        < CONSTANT_PHASE_DURATION
    {
        set_led_parameters(
            hidraw_path,
            &constant_parameters,
        )?;

        constant_count += 1;

        println!(
            "[AMBIENT LIGHTING TEST] Phase A write {:02}: RGB=#4060A0",
            constant_count,
        );

        thread::sleep(
            CONSTANT_OUTPUT_INTERVAL
        );
    }

    println!(
        "[AMBIENT LIGHTING TEST] Phase A complete after {} identical write(s).",
        constant_count
    );

    println!(
        "[AMBIENT LIGHTING TEST] Phase B starting: shader-derived RGB at 2 Hz."
    );

    let sdl =
        sdl2::init()
            .map_err(
                |error| {
                    format!(
                        "SDL initialization failed: {}",
                        error
                    )
                }
            )?;

    let video =
        sdl.video()
            .map_err(
                |error| {
                    format!(
                        "SDL video initialization failed: {}",
                        error
                    )
                }
            )?;

    {
        let gl_attr =
            video.gl_attr();

        gl_attr.set_context_profile(
            GLProfile::Core
        );

        gl_attr.set_context_version(
            crate::define_constants::GL_MAJOR,
            crate::define_constants::GL_MINOR,
        );
    }

    let window =
        video
            .window(
                "Screenshaver Ambient Lighting Test - Phase B",
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
            )
            .position_centered()
            .opengl()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create ambient-lighting test window: {}",
                        error
                    )
                }
            )?;

    let _gl_context =
        window
            .gl_create_context()
            .map_err(
                |error| {
                    format!(
                        "Unable to create ambient-lighting OpenGL context: {}",
                        error
                    )
                }
            )?;

    gl::load_with(
        |symbol| {
            video.gl_get_proc_address(
                symbol
            ) as *const _
        }
    );

    let _ =
        video.gl_set_swap_interval(
            1
        );

    let program =
        crate::compile_shader::build_program(
            crate::define_constants::VERTEX_SHADER,
            TEST_FRAGMENT_SHADER,
        )
        .map_err(
            |error| {
                format!(
                    "Ambient-lighting test shader compilation failed: {}",
                    error
                )
            }
        )?;

    let mut vao =
        0_u32;

    let mut sample_fbo =
        0_u32;

    let mut sample_texture =
        0_u32;

    unsafe {
        gl::GenVertexArrays(
            1,
            &mut vao,
        );

        gl::BindVertexArray(
            vao
        );

        gl::GenFramebuffers(
            1,
            &mut sample_fbo,
        );

        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            sample_fbo,
        );

        gl::GenTextures(
            1,
            &mut sample_texture,
        );

        gl::BindTexture(
            gl::TEXTURE_2D,
            sample_texture,
        );

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB8 as i32,
            SAMPLE_WIDTH as i32,
            SAMPLE_HEIGHT as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            sample_texture,
            0,
        );

        let framebuffer_status =
            gl::CheckFramebufferStatus(
                gl::FRAMEBUFFER
            );

        if framebuffer_status
            != gl::FRAMEBUFFER_COMPLETE
        {
            gl::BindFramebuffer(
                gl::FRAMEBUFFER,
                0,
            );

            gl::DeleteTextures(
                1,
                &sample_texture,
            );

            gl::DeleteFramebuffers(
                1,
                &sample_fbo,
            );

            gl::DeleteVertexArrays(
                1,
                &vao,
            );

            gl::DeleteProgram(
                program
            );

            return Err(
                format!(
                    "Ambient sample framebuffer is incomplete: 0x{:04X}",
                    framebuffer_status
                )
            );
        }

        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            0,
        );
    }

    let time_location =
        uniform_location(
            program,
            b"iTime\0",
        );

    let resolution_location =
        uniform_location(
            program,
            b"iResolution\0",
        );

    let mut event_pump =
        sdl.event_pump()
            .map_err(
                |error| {
                    format!(
                        "Unable to create SDL event pump: {}",
                        error
                    )
                }
            )?;

    let started =
        Instant::now();

    let mut last_sample =
        started
            .checked_sub(
                SAMPLE_INTERVAL
            )
            .unwrap_or(
                started
            );

    let mut last_output =
        started
            .checked_sub(
                SHADER_OUTPUT_INTERVAL
            )
            .unwrap_or(
                started
            );

    let mut pixels =
        vec![
            0u8;
            SAMPLE_WIDTH
                * SAMPLE_HEIGHT
                * 3
        ];

    let mut smoothed:
        Option<[f32; 3]> =
        None;

    let mut output_count =
        0usize;

    let mut stop_requested =
        false;

    while !stop_requested
        && started.elapsed()
            < SHADER_PHASE_DURATION
    {
        for event in
            event_pump.poll_iter()
        {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(
                        Keycode::Escape
                    ),
                    ..
                } => {
                    stop_requested =
                        true;
                }

                _ => {}
            }
        }

        if stop_requested {
            break;
        }

        let elapsed =
            started.elapsed()
                .as_secs_f32();

        unsafe {
            gl::BindFramebuffer(
                gl::FRAMEBUFFER,
                sample_fbo,
            );

            gl::Viewport(
                0,
                0,
                SAMPLE_WIDTH as i32,
                SAMPLE_HEIGHT as i32,
            );

            gl::UseProgram(
                program
            );

            if time_location >= 0 {
                gl::Uniform1f(
                    time_location,
                    elapsed,
                );
            }

            if resolution_location >= 0 {
                gl::Uniform3f(
                    resolution_location,
                    SAMPLE_WIDTH as f32,
                    SAMPLE_HEIGHT as f32,
                    1.0,
                );
            }

            gl::DrawArrays(
                gl::TRIANGLES,
                0,
                3,
            );

            gl::BindFramebuffer(
                gl::READ_FRAMEBUFFER,
                sample_fbo,
            );

            gl::BindFramebuffer(
                gl::DRAW_FRAMEBUFFER,
                0,
            );

            gl::BlitFramebuffer(
                0,
                0,
                SAMPLE_WIDTH as i32,
                SAMPLE_HEIGHT as i32,
                0,
                0,
                WINDOW_WIDTH as i32,
                WINDOW_HEIGHT as i32,
                gl::COLOR_BUFFER_BIT,
                gl::LINEAR,
            );
        }

        window.gl_swap_window();

        let now =
            Instant::now();

        if now.duration_since(
            last_sample
        ) >= SAMPLE_INTERVAL
        {
            unsafe {
                gl::BindFramebuffer(
                    gl::READ_FRAMEBUFFER,
                    sample_fbo,
                );

                gl::PixelStorei(
                    gl::PACK_ALIGNMENT,
                    1,
                );

                gl::ReadPixels(
                    0,
                    0,
                    SAMPLE_WIDTH as i32,
                    SAMPLE_HEIGHT as i32,
                    gl::RGB,
                    gl::UNSIGNED_BYTE,
                    pixels.as_mut_ptr()
                        as *mut _,
                );
            }

            let target =
                representative_color(
                    &pixels
                );

            smoothed =
                Some(
                    match smoothed {
                        Some(previous) => [
                            previous[0]
                                + SMOOTHING_ALPHA
                                    * (
                                        target[0]
                                        - previous[0]
                                    ),
                            previous[1]
                                + SMOOTHING_ALPHA
                                    * (
                                        target[1]
                                        - previous[1]
                                    ),
                            previous[2]
                                + SMOOTHING_ALPHA
                                    * (
                                        target[2]
                                        - previous[2]
                                    ),
                        ],

                        None =>
                            target,
                    }
                );

            last_sample =
                now;
        }

        if now.duration_since(
            last_output
        ) >= SHADER_OUTPUT_INTERVAL
        {
            if let Some(color) =
                smoothed
            {
                let rgb =
                    [
                        float_to_u8(
                            color[0]
                        ),
                        float_to_u8(
                            color[1]
                        ),
                        float_to_u8(
                            color[2]
                        ),
                    ];

                let parameters =
                    LedParameters {
                        mode: 1,
                        speed: 2,
                        brightness:
                            AMBIENT_BRIGHTNESS,
                        option: 0,
                        flags: 7,
                        red: rgb[0],
                        green: rgb[1],
                        blue: rgb[2],
                    };

                set_led_parameters(
                    hidraw_path,
                    &parameters,
                )?;

                output_count += 1;

                println!(
                    "[AMBIENT LIGHTING TEST] Phase B output {:02}: RGB=#{:02X}{:02X}{:02X}",
                    output_count,
                    rgb[0],
                    rgb[1],
                    rgb[2],
                );
            }

            last_output =
                now;
        }

        thread::sleep(
            Duration::from_millis(2)
        );
    }

    unsafe {
        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            0,
        );

        gl::DeleteTextures(
            1,
            &sample_texture,
        );

        gl::DeleteFramebuffers(
            1,
            &sample_fbo,
        );

        gl::DeleteVertexArrays(
            1,
            &vao,
        );

        gl::DeleteProgram(
            program
        );
    }

    println!(
        "[AMBIENT LIGHTING TEST] Phase B ended after {} shader-derived hardware RGB update(s).",
        output_count
    );

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


fn discover_target_hidraw_devices(
) -> Result<Vec<CandidateDevice>, String> {

    let hidraw_root =
        Path::new(
            "/sys/class/hidraw"
        );


    let entries =
        fs::read_dir(
            hidraw_root
        )
        .map_err(
            |error| {
                format!(
                    "Unable to enumerate {}: {}",
                    hidraw_root.display(),
                    error,
                )
            }
        )?;


    let mut candidates =
        Vec::new();


    for entry in entries {

        let entry =
            entry.map_err(
                |error| {
                    format!(
                        "Unable to read hidraw sysfs entry: {}",
                        error
                    )
                }
            )?;


        let hidraw_name =
            entry.file_name();


        let hidraw_name =
            hidraw_name
                .to_string_lossy()
                .to_string();


        let device_link =
            entry.path()
                .join(
                    "device"
                );


        let canonical_device =
            match fs::canonicalize(
                &device_link
            ) {
                Ok(path) => path,
                Err(_) => continue,
            };


        let canonical_text =
            canonical_device
                .to_string_lossy()
                .to_ascii_lowercase();


        let usb_marker =
            format!(
                "{TARGET_VENDOR_ID}:{TARGET_PRODUCT_ID}"
            );


        if !canonical_text.contains(
            &usb_marker
        ) {
            continue;
        }


        let interface_number =
            match interface_number_from_path(
                &canonical_device
            ) {
                Some(value) => value,
                None => continue,
            };


        let report_descriptor_path =
            device_link.join(
                "report_descriptor"
            );


        let report_descriptor =
            fs::read(
                &report_descriptor_path
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to read {}: {}",
                        report_descriptor_path.display(),
                        error,
                    )
                }
            )?;


        candidates.push(
            CandidateDevice {
                hidraw_path:
                    PathBuf::from(
                        "/dev"
                    )
                    .join(
                        hidraw_name
                    ),

                interface_number,

                report_descriptor,
            }
        );
    }


    candidates.sort_by(
        |left, right| {
            left.interface_number
                .cmp(
                    &right.interface_number
                )
        }
    );


    Ok(
        candidates
    )
}


fn interface_number_from_path(
    path: &Path,
) -> Option<u8> {

    for component in path.components() {

        let text =
            component
                .as_os_str()
                .to_string_lossy();


        if let Some(
            (_, interface_text)
        ) = text.rsplit_once(
            ":1."
        ) {

            if interface_text
                .chars()
                .all(
                    |character| {
                        character.is_ascii_digit()
                    }
                )
            {

                if let Ok(
                    value
                ) = interface_text.parse::<u8>() {

                    return Some(
                        value
                    );
                }
            }
        }
    }


    None
}


fn is_expected_vendor_feature_descriptor(
    descriptor: &[u8],
) -> bool {

    const EXPECTED: [u8; 20] = [
        0x06, 0xFF, 0xFF,
        0x09, 0x02,
        0xA1, 0x01,
        0x09, 0x02,
        0x15, 0x80,
        0x25, 0x7F,
        0x95, 0x40,
        0x75, 0x08,
        0xB1, 0x02,
        0xC0,
    ];


    descriptor
        == EXPECTED
}


fn identify_device(
    hidraw_path: &Path,
) -> Result<u32, String> {

    let file =
        OpenOptions::new()
            .read(
                true
            )
            .write(
                true
            )
            .open(
                hidraw_path
            )
            .map_err(
                |error| {
                    if error.kind()
                        == io::ErrorKind::PermissionDenied
                    {
                        format!(
                            "Permission denied opening {}. The harness has \
                             not sent any HID command. A NixOS udev rule will \
                             be required before non-root ambient-lighting \
                             access can be tested.",
                            hidraw_path.display(),
                        )
                    } else {
                        format!(
                            "Unable to open {}: {}",
                            hidraw_path.display(),
                            error,
                        )
                    }
                }
            )?;


    let fd =
        file.as_raw_fd();


    // HIDAPI-style hidraw Feature Reports include an extra leading report-ID
    // byte.  The TK75-TMR vendor collection has no numbered reports, so byte 0
    // is 0 and the 64-byte protocol packet begins at byte 1.
    let mut request =
        [0u8; FEATURE_REPORT_BUFFER_LENGTH];

    request[0] = 0;
    request[1] = IDENTIFY_OPCODE;

    // 0x8F uses the protocol's Bit7 checksum: protocol bytes 0..=6 are
    // summed and protocol byte 7 stores 0xFF - (sum & 0xFF).  The leading
    // HID report-ID byte shifts protocol byte 7 to request[8].
    request[8] =
        bit7_checksum(
            &request[1..=7]
        );


    let send_request =
        hid_iocsfeature(
            FEATURE_REPORT_BUFFER_LENGTH
        );


    let send_result =
        unsafe {
            ioctl(
                fd,
                send_request,
                request.as_mut_ptr(),
            )
        };


    if send_result < 0 {

        return Err(
            format!(
                "Unable to send the documented 0x{:02X} identify request to {}: {}",
                IDENTIFY_OPCODE,
                hidraw_path.display(),
                io::Error::last_os_error(),
            )
        );
    }


    let mut response =
        [0u8; FEATURE_REPORT_BUFFER_LENGTH];

    response[0] = 0;


    let get_request =
        hid_iocgfeature(
            FEATURE_REPORT_BUFFER_LENGTH
        );


    let get_result =
        unsafe {
            ioctl(
                fd,
                get_request,
                response.as_mut_ptr(),
            )
        };


    if get_result < 0 {

        return Err(
            format!(
                "The identify request was accepted, but its Feature Report \
                 response could not be read from {}: {}",
                hidraw_path.display(),
                io::Error::last_os_error(),
            )
        );
    }


    // response[0] is the HID report ID.  Protocol byte 0 is response[1].
    if response[1]
        != IDENTIFY_OPCODE
    {

        return Err(
            format!(
                "Unexpected identify response opcode 0x{:02X}; expected 0x{:02X}. \
                 No further command was sent.",
                response[1],
                IDENTIFY_OPCODE,
            )
        );
    }


    let device_id =
        u32::from_le_bytes(
            [
                response[2],
                response[3],
                response[4],
                response[5],
            ]
        );


    Ok(
        device_id
    )
}


fn read_led_parameters(
    hidraw_path: &Path,
) -> Result<LedParameters, String> {

    let file =
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(hidraw_path)
            .map_err(
                |error| {
                    format!(
                        "Unable to reopen {} for the read-only LEDPARAM request: {}",
                        hidraw_path.display(),
                        error,
                    )
                }
            )?;


    let fd =
        file.as_raw_fd();


    let mut request =
        [0u8; FEATURE_REPORT_BUFFER_LENGTH];

    request[0] = 0;
    request[1] = GET_LEDPARAM_OPCODE;

    // GET_LEDPARAM is an ordinary app-mode GET and therefore uses the
    // protocol's Bit7 checksum: protocol bytes 0..=6 are summed and
    // protocol byte 7 stores 0xFF - (sum & 0xFF).
    request[8] =
        bit7_checksum(
            &request[1..=7]
        );


    let send_result =
        unsafe {
            ioctl(
                fd,
                hid_iocsfeature(
                    FEATURE_REPORT_BUFFER_LENGTH
                ),
                request.as_mut_ptr(),
            )
        };


    if send_result < 0 {

        return Err(
            format!(
                "Unable to send the documented 0x{:02X} GET_LEDPARAM request to {}: {}",
                GET_LEDPARAM_OPCODE,
                hidraw_path.display(),
                io::Error::last_os_error(),
            )
        );
    }


    let mut response =
        [0u8; FEATURE_REPORT_BUFFER_LENGTH];

    response[0] = 0;


    let get_result =
        unsafe {
            ioctl(
                fd,
                hid_iocgfeature(
                    FEATURE_REPORT_BUFFER_LENGTH
                ),
                response.as_mut_ptr(),
            )
        };


    if get_result < 0 {

        return Err(
            format!(
                "The GET_LEDPARAM request was accepted, but its Feature Report response could not be read from {}: {}",
                hidraw_path.display(),
                io::Error::last_os_error(),
            )
        );
    }


    println!(
        "[AMBIENT LIGHTING TEST] Raw GET_LEDPARAM Feature Report response (64 protocol bytes):"
    );

    for (row_index, chunk) in response[1..].chunks(16).enumerate() {
        let start = row_index * 16;
        let end = start + chunk.len() - 1;

        let hex =
            chunk
                .iter()
                .map(|value| format!("{:02X}", value))
                .collect::<Vec<_>>()
                .join(" ");

        println!(
            "[AMBIENT LIGHTING TEST]   {:02}-{:02}: {}",
            start,
            end,
            hex,
        );
    }


    if response[1] != GET_LEDPARAM_OPCODE {

        return Err(
            format!(
                "Unexpected LEDPARAM response opcode 0x{:02X}; expected 0x{:02X}. No lighting-setting command was sent.",
                response[1],
                GET_LEDPARAM_OPCODE,
            )
        );
    }


    // GET_LEDPARAM replies echo the opcode and return the lighting fields in
    // protocol bytes 1..7.  Real TK75-TMR hardware returns zero in byte 8;
    // Bit8 is required for SET_LEDPARAM writes, not validated on this GET reply.
    let option_and_flags =
        response[5];

    Ok(
        LedParameters {
            mode: response[2],
            speed: response[3],
            brightness: response[4],
            option: option_and_flags >> 4,
            flags: option_and_flags & 0x0F,
            red: response[6],
            green: response[7],
            blue: response[8],
        }
    )
}


fn set_led_parameters(
    hidraw_path: &Path,
    parameters: &LedParameters,
) -> Result<(), String> {

    let file =
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(hidraw_path)
            .map_err(|error| {
                format!(
                    "Unable to open {} for SET_LEDPARAM: {}",
                    hidraw_path.display(),
                    error,
                )
            })?;

    let fd = file.as_raw_fd();
    let mut request = [0u8; FEATURE_REPORT_BUFFER_LENGTH];

    request[0] = 0;
    request[1] = SET_LEDPARAM_OPCODE;
    request[2] = parameters.mode;
    request[3] = parameters.speed;
    request[4] = parameters.brightness;
    request[5] = (parameters.option << 4) | (parameters.flags & 0x0F);
    request[6] = parameters.red;
    request[7] = parameters.green;
    request[8] = parameters.blue;

    // SET_LEDPARAM uses the protocol Bit8 checksum: protocol bytes 0..=7
    // are summed and protocol byte 8 stores 0xFF - (sum & 0xFF).
    request[9] =
        0xFFu8.wrapping_sub(
            request[1..=8]
                .iter()
                .fold(0u8, |sum, value| sum.wrapping_add(*value))
        );

    let send_result =
        unsafe {
            ioctl(
                fd,
                hid_iocsfeature(FEATURE_REPORT_BUFFER_LENGTH),
                request.as_mut_ptr(),
            )
        };

    if send_result < 0 {
        return Err(
            format!(
                "Unable to send SET_LEDPARAM to {}: {}",
                hidraw_path.display(),
                io::Error::last_os_error(),
            )
        );
    }

    Ok(())
}


fn print_led_parameters(
    label: &str,
    parameters: &LedParameters,
) {
    println!(
        "[AMBIENT LIGHTING TEST] {} LEDPARAM: mode={}, speed={}, brightness={}, option={}, flags={} (0x{:X}), RGB=#{:02X}{:02X}{:02X}",
        label,
        parameters.mode,
        parameters.speed,
        parameters.brightness,
        parameters.option,
        parameters.flags,
        parameters.flags,
        parameters.red,
        parameters.green,
        parameters.blue,
    );
}


fn led_parameters_equal(
    left: &LedParameters,
    right: &LedParameters,
) -> bool {
    left.mode == right.mode
        && left.speed == right.speed
        && left.brightness == right.brightness
        && left.option == right.option
        && left.flags == right.flags
        && left.red == right.red
        && left.green == right.green
        && left.blue == right.blue
}


fn bit7_checksum(
    bytes_zero_through_six: &[u8],
) -> u8 {

    0xFFu8.wrapping_sub(
        bytes_zero_through_six
            .iter()
            .fold(
                0u8,
                |sum, value| {
                    sum.wrapping_add(*value)
                }
            )
    )
}


fn classify_tk75_tmr_device_id(
    device_id: u32,
) -> Option<&'static str> {

    match device_id {
        3590 => Some(
            "device 3590"
        ),

        3591 => Some(
            "device 3591"
        ),

        4226 => Some(
            "device 4226"
        ),

        4227 => Some(
            "device 4227"
        ),

        _ => None,
    }
}


const fn hid_ioc(
    direction: u32,
    ioctl_type: u32,
    number: u32,
    size: usize,
) -> usize {

    (
        (direction << IOC_DIRSHIFT)
        | (ioctl_type << IOC_TYPESHIFT)
        | (number << IOC_NRSHIFT)
        | ((size as u32) << IOC_SIZESHIFT)
    ) as usize
}


const fn hid_iocsfeature(
    size: usize,
) -> usize {

    hid_ioc(
        IOC_WRITE | IOC_READ,
        HIDRAW_IOCTL_TYPE,
        HIDIOCSFEATURE_NR,
        size,
    )
}


const fn hid_iocgfeature(
    size: usize,
) -> usize {

    hid_ioc(
        IOC_WRITE | IOC_READ,
        HIDRAW_IOCTL_TYPE,
        HIDIOCGFEATURE_NR,
        size,
    )
}
