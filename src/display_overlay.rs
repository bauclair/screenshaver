use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};

pub fn display(
    canvas: &mut Canvas<Window>,
    texture_creator: &TextureCreator<WindowContext>,
    overlay: &crate::construct_text_overlay::ConstructedTextOverlay,
    placement: crate::parse_subtitle_placement::SubtitlePlacement,
    output_width: u32,
    output_height: u32,
) -> Result<(), String> {
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::RGBA32,
            overlay.width,
            overlay.height,
        )
        .map_err(|error| format!("Unable to create SDL subtitle texture: {}", error))?;

    texture.set_blend_mode(sdl2::render::BlendMode::Blend);

    let pitch = usize::try_from(overlay.width)
        .map_err(|_| "Subtitle width cannot be represented as usize".to_string())?
        .checked_mul(4)
        .ok_or_else(|| "Subtitle pitch overflow".to_string())?;

    texture
        .update(None, &overlay.pixels, pitch)
        .map_err(|error| format!("Unable to upload subtitle pixels: {}", error))?;

    canvas
        .copy(
            &texture,
            None,
            destination_rect(
                overlay.width,
                overlay.height,
                placement,
                output_width,
                output_height,
            ),
        )
        .map_err(|error| format!("Unable to draw subtitle overlay: {}", error))
}

fn destination_rect(
    overlay_width: u32,
    overlay_height: u32,
    placement: crate::parse_subtitle_placement::SubtitlePlacement,
    output_width: u32,
    output_height: u32,
) -> Rect {
    let margin = crate::construct_text_overlay::edge_margin(output_height);

    let x = match placement.horizontal {
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Left => margin,
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Center => {
            output_width.saturating_sub(overlay_width) / 2
        }
        crate::parse_subtitle_placement::SubtitleHorizontalPosition::Right => {
            output_width.saturating_sub(overlay_width.saturating_add(margin))
        }
    };

    let y = match placement.vertical {
        crate::parse_subtitle_placement::SubtitleVerticalPosition::Top => margin,
        crate::parse_subtitle_placement::SubtitleVerticalPosition::Bottom => {
            output_height.saturating_sub(overlay_height.saturating_add(margin))
        }
    };

    Rect::new(x as i32, y as i32, overlay_width, overlay_height)
}

