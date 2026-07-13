//! Display an in-memory procedural texture using SDL2.
//!
//! This module is intentionally independent of the normal
//! OpenGL shader renderer. Its purpose is to provide a simple
//! diagnostic display path for generated RGBA8 textures.

use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;


/// Display one generated texture until the user provides input.
///
/// The preview closes on:
///
/// - any key press;
/// - any mouse button press;
/// - meaningful mouse movement;
/// - window close;
/// - SDL quit event.
pub fn display(
    generated:
        &crate::generate_textures::GeneratedTexture,
) -> Result<(), String> {

    generated
        .validate_standard()
        .map_err(
            |error| {
                format!(
                    "Cannot display invalid generated texture: {}",
                    error
                )
            }
        )?;


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


    let display_mode =
        video
            .current_display_mode(
                0
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to query display mode: {}",
                        error
                    )
                }
            )?;


    let display_width =
        display_mode.w.max(
            1
        ) as u32;


    let display_height =
        display_mode.h.max(
            1
        ) as u32;


    let preview_size =
        calculate_preview_size(
            display_width,
            display_height,
            generated.width,
            generated.height,
        );


    let window =
        video
            .window(
                "Screenshaver Texture Preview",
                preview_size.0,
                preview_size.1,
            )
            .position_centered()
            .resizable()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create texture preview window: {}",
                        error
                    )
                }
            )?;


    let mut canvas =
        window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(
                |error| {
                    format!(
                        "Unable to create SDL texture-preview canvas: {}",
                        error
                    )
                }
            )?;


    let texture_creator =
        canvas.texture_creator();


    let mut texture =
        texture_creator
            .create_texture_streaming(
                PixelFormatEnum::RGBA32,
                generated.width,
                generated.height,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to create SDL streaming texture: {}",
                        error
                    )
                }
            )?;


    let pitch =
        usize::try_from(
            generated.width
        )
        .map_err(
            |_| {
                "Generated texture width cannot be represented as usize"
                    .to_string()
            }
        )?
        .checked_mul(
            crate::generate_textures::CHANNELS_PER_PIXEL
        )
        .ok_or_else(
            || {
                "Generated texture pitch overflow"
                    .to_string()
            }
        )?;


    texture
        .update(
            None,
            &generated.pixels,
            pitch,
        )
        .map_err(
            |error| {
                format!(
                    "Unable to upload generated pixels to SDL texture: {}",
                    error
                )
            }
        )?;


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


    // ------------------------------------------------------------
    // Ignore startup input
    //
    // The terminal Enter key used to launch Screenshaver may still
    // be present in SDL's event queue when the preview window opens.
    // Discard events briefly before arming preview-close behavior.
    // ------------------------------------------------------------

    let input_arm_time =
        std::time::Instant::now()
            + Duration::from_millis(
                500
            );


    while std::time::Instant::now()
        < input_arm_time
    {
        for _event in
            event_pump.poll_iter()
        {
            // Intentionally discarded.
        }


        std::thread::sleep(
            Duration::from_millis(
                10
            )
        );
    }            


let mouse_threshold =
    8_i32;

let mut accumulated_mouse_x =
    0_i32;

let mut accumulated_mouse_y =
    0_i32;


    loop {

        for event in
            event_pump.poll_iter()
        {
            match event {

                Event::Quit {
                    ..
                } => {

                    return Ok(());
                }


                Event::KeyDown {
                    keycode:
                        Some(
                            Keycode::Escape
                        ),
                    ..
                } => {

                    return Ok(());
                }


                Event::KeyDown {
                    ..
                } => {

                    return Ok(());
                }


                Event::MouseButtonDown {
                    ..
                } => {

                    return Ok(());
                }


                Event::MouseMotion {
                    xrel,
                    yrel,
                    ..
                } => {

                    accumulated_mouse_x +=
                        xrel.abs();

                    accumulated_mouse_y +=
                        yrel.abs();


                    if accumulated_mouse_x
                        >= mouse_threshold
                        || accumulated_mouse_y
                            >= mouse_threshold
                    {
                        return Ok(());
                    }
                }

                _ => {}
            }
        }


        let (
            output_width,
            output_height,
        ) =
            canvas
                .output_size()
                .map_err(
                    |error| {
                        format!(
                            "Unable to query preview output size: {}",
                            error
                        )
                    }
                )?;


        let destination =
            fit_texture_to_window(
                output_width,
                output_height,
                generated.width,
                generated.height,
            );


        canvas.set_draw_color(
            sdl2::pixels::Color::RGB(
                0,
                0,
                0,
            )
        );


        canvas.clear();


        canvas
            .copy(
                &texture,
                None,
                destination,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to draw generated texture: {}",
                        error
                    )
                }
            )?;


        canvas.present();


        std::thread::sleep(
            Duration::from_millis(
                16
            )
        );
    }
}


// ============================================================
// Window sizing
// ============================================================

fn calculate_preview_size(
    display_width: u32,
    display_height: u32,
    texture_width: u32,
    texture_height: u32,
) -> (
    u32,
    u32,
) {

    let maximum_width =
        (
            display_width as f32
                * 0.80
        )
        .round()
        .max(
            320.0
        ) as u32;


    let maximum_height =
        (
            display_height as f32
                * 0.80
        )
        .round()
        .max(
            320.0
        ) as u32;


    fit_size(
        maximum_width,
        maximum_height,
        texture_width,
        texture_height,
    )
}


fn fit_texture_to_window(
    window_width: u32,
    window_height: u32,
    texture_width: u32,
    texture_height: u32,
) -> Option<Rect> {

    let (
        width,
        height,
    ) =
        fit_size(
            window_width,
            window_height,
            texture_width,
            texture_height,
        );


    let x =
        (
            window_width
                .saturating_sub(
                    width
                )
            / 2
        ) as i32;


    let y =
        (
            window_height
                .saturating_sub(
                    height
                )
            / 2
        ) as i32;


    Some(
        Rect::new(
            x,
            y,
            width,
            height,
        )
    )
}


fn fit_size(
    maximum_width: u32,
    maximum_height: u32,
    texture_width: u32,
    texture_height: u32,
) -> (
    u32,
    u32,
) {

    if texture_width
        == 0
        || texture_height
            == 0
    {
        return (
            1,
            1,
        );
    }


    let width_scale =
        maximum_width as f64
            / texture_width as f64;


    let height_scale =
        maximum_height as f64
            / texture_height as f64;


    let scale =
        width_scale
            .min(
                height_scale
            )
            .min(
                1.0
            );


    let width =
        (
            texture_width as f64
                * scale
        )
        .round()
        .max(
            1.0
        ) as u32;


    let height =
        (
            texture_height as f64
                * scale
        )
        .round()
        .max(
            1.0
        ) as u32;


    (
        width,
        height,
    )
}