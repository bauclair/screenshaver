use image::ImageReader;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{
    Color,
    PixelFormatEnum,
};

use std::io::Cursor;
use std::time::{
    Duration,
    Instant,
};


//
// ------------------------------------------------------------
// Embedded splash-screen artwork
// ------------------------------------------------------------
//

const SPLASH_IMAGE: &[u8] =
    include_bytes!(
        "../assets/screenshaver-splash.png"
    );


//
// ------------------------------------------------------------
// Static display settings
// ------------------------------------------------------------
//

const SPLASH_DISPLAY_TIME:
    Duration =
        Duration::from_millis(2500);


const SPLASH_FRAME_DELAY:
    Duration =
        Duration::from_millis(16);


const MAX_SCREEN_WIDTH_PERCENT:
    f32 =
        0.70;


const MAX_SCREEN_HEIGHT_PERCENT:
    f32 =
        0.70;


const MAX_SPLASH_WIDTH:
    u32 =
        900;


//
// ------------------------------------------------------------
// Display splash screen
// ------------------------------------------------------------
//

pub fn show_splash(
    sdl: &sdl2::Sdl,
) -> Result<(), String> {

    //---------------------------------------------------------
    // Decode embedded PNG
    //---------------------------------------------------------

    let reader =
        ImageReader::new(
            Cursor::new(
                SPLASH_IMAGE
            )
        )
        .with_guessed_format()
        .map_err(|error| {

            format!(
                "Unable to determine splash image format: {}",
                error,
            )
        })?;


    let decoded =
        reader.decode()
            .map_err(|error| {

                format!(
                    "Unable to decode splash image: {}",
                    error,
                )
            })?;


    let rgba_image =
        decoded.to_rgba8();


    let image_width =
        rgba_image.width();


    let image_height =
        rgba_image.height();


    if image_width == 0
        || image_height == 0
    {
        return Err(
            "Splash image has invalid dimensions"
                .to_string()
        );
    }


    //---------------------------------------------------------
    // Initialize SDL video subsystem
    //---------------------------------------------------------

    let video =
        sdl.video()
            .map_err(|error| {

                format!(
                    "Unable to initialize SDL video for splash screen: {}",
                    error,
                )
            })?;


    //---------------------------------------------------------
    // Determine usable display dimensions
    //---------------------------------------------------------

    let display_bounds =
        video.display_usable_bounds(0)
            .or_else(|_| {

                video.display_bounds(0)
            })
            .map_err(|error| {

                format!(
                    "Unable to determine display dimensions: {}",
                    error,
                )
            })?;


    let maximum_width =
        (
            display_bounds.width() as f32
                * MAX_SCREEN_WIDTH_PERCENT
        ) as u32;


    let maximum_height =
        (
            display_bounds.height() as f32
                * MAX_SCREEN_HEIGHT_PERCENT
        ) as u32;


    let width_scale =
        maximum_width as f32
            / image_width as f32;


    let height_scale =
        maximum_height as f32
            / image_height as f32;


    let configured_width_scale =
        MAX_SPLASH_WIDTH as f32
            / image_width as f32;


    let scale =
        1.0_f32
            .min(width_scale)
            .min(height_scale)
            .min(configured_width_scale);


    let window_width =
        (
            image_width as f32
                * scale
        )
        .round()
        .max(1.0) as u32;


    let window_height =
        (
            image_height as f32
                * scale
        )
        .round()
        .max(1.0) as u32;


    //---------------------------------------------------------
    // Create temporary borderless window
    //---------------------------------------------------------

    let window =
        video.window(
            "Screenshaver",
            window_width,
            window_height,
        )
        .position_centered()
        .borderless()
        .build()
        .map_err(|error| {

            format!(
                "Unable to create splash window: {}",
                error,
            )
        })?;


    //---------------------------------------------------------
    // Create SDL canvas
    //---------------------------------------------------------

    let mut canvas =
        window.into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|error| {

                format!(
                    "Unable to create splash canvas: {}",
                    error,
                )
            })?;


    canvas.set_draw_color(
        Color::RGB(
            0,
            0,
            0,
        )
    );


    canvas.clear();


    //---------------------------------------------------------
    // Create texture from decoded PNG pixels
    //---------------------------------------------------------

    let texture_creator =
        canvas.texture_creator();


    let mut texture =
        texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGBA32,
                image_width,
                image_height,
            )
            .map_err(|error| {

                format!(
                    "Unable to create splash texture: {}",
                    error,
                )
            })?;


    texture.update(
        None,
        rgba_image.as_raw(),
        (
            image_width
                * 4
        ) as usize,
    )
    .map_err(|error| {

        format!(
            "Unable to upload splash texture: {}",
            error,
        )
    })?;


    //---------------------------------------------------------
    // Draw splash image
    //---------------------------------------------------------

    canvas.copy(
        &texture,
        None,
        None,
    )
    .map_err(|error| {

        format!(
            "Unable to draw splash texture: {}",
            error,
        )
    })?;


    canvas.present();


    //---------------------------------------------------------
    // Keep window responsive for static display duration
    //---------------------------------------------------------

    let mut event_pump =
        sdl.event_pump()
            .map_err(|error| {

                format!(
                    "Unable to create splash event pump: {}",
                    error,
                )
            })?;


    let start_time =
        Instant::now();


    while start_time.elapsed()
        < SPLASH_DISPLAY_TIME
    {
        for event in
            event_pump.poll_iter()
        {
            match event {

                Event::Quit { .. }

                | Event::KeyDown {
                    keycode:
                        Some(
                            Keycode::Escape
                        ),
                    ..
                } => {

                    return Ok(());
                }


                _ => {}
            }
        }


        std::thread::sleep(
            SPLASH_FRAME_DELAY
        );
    }


    Ok(())
}