use std::path::{Path, PathBuf};
use std::process::Command;

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::ttf::FontStyle;

pub use crate::fps_monitor::FpsWarningState;

const BASELINE_HEIGHT: f32 = 1080.0;
const MINIMUM_SCALE: f32 = 0.75;
const MAXIMUM_SCALE: f32 = 2.0;
const BASE_FONT_SIZE: f32 = 18.0;
const BASE_HORIZONTAL_PADDING: f32 = 16.0;
const BASE_VERTICAL_PADDING: f32 = 9.0;
const MAXIMUM_WIDTH_RATIO: f32 = 0.80;
const BACKGROUND_RGBA: [u8; 4] = [0, 0, 0, 150];
const TEXT_COLOR: Color = Color::RGBA(245, 245, 245, 255);
const FPS_WARNING_COLOR: Color = Color::RGBA(255, 221, 64, 255);
const FPS_CRITICAL_COLOR: Color = Color::RGBA(255, 72, 72, 255);

#[derive(Debug, Clone, Default)]
pub struct OverlayDescriptor {
    pub shader: Option<String>,
    pub texture: Option<String>,
    pub palette: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConstructedTextOverlay {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub fn construct(
    descriptor: &OverlayDescriptor,
    output_width: u32,
    output_height: u32,
) -> Result<ConstructedTextOverlay, String> {
    construct_with_fps(
        descriptor,
        None,
        output_width,
        output_height,
    )
}

pub fn construct_with_fps(
    descriptor: &OverlayDescriptor,
    rendered_fps: Option<u32>,
    output_width: u32,
    output_height: u32,
) -> Result<ConstructedTextOverlay, String> {
    construct_with_fps_warning(
        descriptor,
        rendered_fps,
        FpsWarningState::Normal,
        output_width,
        output_height,
    )
}

pub fn construct_with_fps_warning(
    descriptor: &OverlayDescriptor,
    rendered_fps: Option<u32>,
    warning_state: FpsWarningState,
    output_width: u32,
    output_height: u32,
) -> Result<ConstructedTextOverlay, String> {
    let scale = calculate_scale(output_height);
    let font_size = (BASE_FONT_SIZE * scale).round().clamp(10.0, 72.0) as u16;
    let padding_x = (BASE_HORIZONTAL_PADDING * scale).round().max(8.0) as u32;
    let padding_y = (BASE_VERTICAL_PADDING * scale).round().max(5.0) as u32;
    let maximum_width = (output_width as f32 * MAXIMUM_WIDTH_RATIO)
        .round()
        .max(1.0) as u32;

    let ttf_context = sdl2::ttf::init()
        .map_err(|error| format!("Unable to initialize SDL_ttf: {}", error))?;
    let font_path = locate_subtitle_font()?;
    let normal_font = ttf_context
        .load_font(&font_path, font_size)
        .map_err(|error| format!(
            "Unable to load subtitle font '{}': {}",
            font_path.display(),
            error
        ))?;
    let mut critical_font = ttf_context
        .load_font(&font_path, font_size)
        .map_err(|error| format!(
            "Unable to load bold subtitle font '{}': {}",
            font_path.display(),
            error
        ))?;
    critical_font.set_style(FontStyle::BOLD);

    let fps_font = if matches!(
        warning_state,
        FpsWarningState::Critical
            | FpsWarningState::CriticalHidden
    ) {
        &critical_font
    } else {
        &normal_font
    };

    let available_text_width = maximum_width
        .saturating_sub(padding_x.saturating_mul(2))
        .max(1);
    let text = fit_descriptor_text(
        descriptor,
        rendered_fps,
        &normal_font,
        fps_font,
        available_text_width,
    )?;

    if text.is_empty() {
        return Err("Cannot construct a text overlay without content".to_string());
    }

    let (prefix_text, fps_text) = split_fps_segment(&text, rendered_fps);

    let prefix_surface = if prefix_text.is_empty() {
        None
    } else {
        Some(
            normal_font
                .render(prefix_text)
                .blended(TEXT_COLOR)
                .map_err(|error| format!("Unable to render subtitle text: {}", error))?
                .convert_format(PixelFormatEnum::RGBA32)
                .map_err(|error| format!("Unable to convert subtitle text surface: {}", error))?,
        )
    };

    let fps_color = match warning_state {
        FpsWarningState::Normal => TEXT_COLOR,
        FpsWarningState::Warning => FPS_WARNING_COLOR,
        FpsWarningState::Critical
        | FpsWarningState::CriticalHidden => FPS_CRITICAL_COLOR,
    };
    let fps_surface = if fps_text.is_empty() {
        None
    } else {
        Some(
            fps_font
                .render(fps_text)
                .blended(fps_color)
                .map_err(|error| format!("Unable to render FPS subtitle text: {}", error))?
                .convert_format(PixelFormatEnum::RGBA32)
                .map_err(|error| format!("Unable to convert FPS subtitle surface: {}", error))?,
        )
    };

    let prefix_width = prefix_surface.as_ref().map_or(0, |surface| surface.width());
    let fps_width = fps_surface.as_ref().map_or(0, |surface| surface.width());
    let text_width = prefix_width.saturating_add(fps_width);
    let text_height = prefix_surface
        .as_ref()
        .map_or(0, |surface| surface.height())
        .max(fps_surface.as_ref().map_or(0, |surface| surface.height()));
    let height = text_height.saturating_add(padding_y.saturating_mul(2)).max(1);
    let width = text_width
        .saturating_add(padding_x.saturating_mul(2))
        .max(height)
        .min(maximum_width)
        .max(1);

    let byte_count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Subtitle overlay dimensions overflow".to_string())?;

    let mut pixels = vec![0_u8; byte_count];
    draw_capsule_background(&mut pixels, width, height);

    if let Some(surface) = prefix_surface.as_ref() {
        composite_surface(
            &mut pixels,
            width,
            height,
            surface,
            padding_x,
            padding_y.saturating_add((text_height.saturating_sub(surface.height())) / 2),
        )?;
    }

    if warning_state != FpsWarningState::CriticalHidden {
        if let Some(surface) = fps_surface.as_ref() {
            composite_surface(
                &mut pixels,
                width,
                height,
                surface,
                padding_x.saturating_add(prefix_width),
                padding_y.saturating_add((text_height.saturating_sub(surface.height())) / 2),
            )?;
        }
    }

    Ok(ConstructedTextOverlay {
        width,
        height,
        pixels,
    })
}


pub fn construct_message(
    message: &str,
    output_width: u32,
    output_height: u32,
) -> Result<ConstructedTextOverlay, String> {
    let message =
        message.trim();


    if message.is_empty() {
        return Err(
            "Cannot construct a message overlay without content"
                .to_string()
        );
    }


    let scale =
        calculate_scale(
            output_height
        );

    let font_size =
        (
            BASE_FONT_SIZE
                * scale
        )
        .round()
        .clamp(
            10.0,
            72.0,
        ) as u16;

    let padding_x =
        (
            BASE_HORIZONTAL_PADDING
                * scale
        )
        .round()
        .max(
            8.0
        ) as u32;

    let padding_y =
        (
            BASE_VERTICAL_PADDING
                * scale
        )
        .round()
        .max(
            5.0
        ) as u32;

    let maximum_width =
        (
            output_width as f32
                * MAXIMUM_WIDTH_RATIO
        )
        .round()
        .max(
            1.0
        ) as u32;


    let ttf_context =
        sdl2::ttf::init()
            .map_err(
                |error| {
                    format!(
                        "Unable to initialize SDL_ttf: {}",
                        error,
                    )
                }
            )?;


    let font_path =
        locate_subtitle_font()?;


    let font =
        ttf_context
            .load_font(
                &font_path,
                font_size,
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to load message-overlay font '{}': {}",
                        font_path.display(),
                        error,
                    )
                }
            )?;


    let available_text_width =
        maximum_width
            .saturating_sub(
                padding_x.saturating_mul(
                    2
                )
            )
            .max(
                1
            );


    let fitted_message =
        if text_fits(
            &font,
            message,
            available_text_width,
        )? {
            message.to_string()
        } else {
            truncate_plain_text(
                message,
                &font,
                available_text_width,
            )?
        };


    let text_surface =
        font
            .render(
                &fitted_message
            )
            .blended(
                TEXT_COLOR
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to render message-overlay text: {}",
                        error,
                    )
                }
            )?
            .convert_format(
                PixelFormatEnum::RGBA32
            )
            .map_err(
                |error| {
                    format!(
                        "Unable to convert message-overlay text surface: {}",
                        error,
                    )
                }
            )?;


    let text_width =
        text_surface.width();

    let text_height =
        text_surface.height();

    let height =
        text_height
            .saturating_add(
                padding_y.saturating_mul(
                    2
                )
            )
            .max(
                1
            );

    let width =
        text_width
            .saturating_add(
                padding_x.saturating_mul(
                    2
                )
            )
            .max(
                height
            )
            .min(
                maximum_width
            )
            .max(
                1
            );


    let byte_count =
        usize::try_from(
            width
        )
        .ok()
        .and_then(
            |w| {
                usize::try_from(
                    height
                )
                .ok()
                .and_then(
                    |h| {
                        w.checked_mul(
                            h
                        )
                    }
                )
            }
        )
        .and_then(
            |pixels| {
                pixels.checked_mul(
                    4
                )
            }
        )
        .ok_or_else(
            || {
                "Message overlay dimensions overflow"
                    .to_string()
            }
        )?;


    let mut pixels =
        vec![
            0_u8;
            byte_count
        ];


    draw_capsule_background(
        &mut pixels,
        width,
        height,
    );


    composite_surface(
        &mut pixels,
        width,
        height,
        &text_surface,
        padding_x,
        padding_y
            .saturating_add(
                (
                    text_height
                        .saturating_sub(
                            text_surface.height()
                        )
                )
                    / 2
            ),
    )?;


    Ok(
        ConstructedTextOverlay {
            width,
            height,
            pixels,
        }
    )
}

pub fn calculate_scale(output_height: u32) -> f32 {
    (output_height as f32 / BASELINE_HEIGHT).clamp(MINIMUM_SCALE, MAXIMUM_SCALE)
}

pub fn edge_margin(output_height: u32) -> u32 {
    (24.0 * calculate_scale(output_height)).round().max(12.0) as u32
}

fn format_descriptor(
    descriptor: &OverlayDescriptor,
    rendered_fps: Option<u32>,
) -> String {
    let mut fields = Vec::new();

    if let Some(value) = nonempty(descriptor.shader.as_deref()) {
        fields.push(format!("P: {}", value));
    }
    if let Some(value) = nonempty(descriptor.texture.as_deref()) {
        fields.push(format!("T: {}", value));
    }
    if let Some(value) = nonempty(descriptor.palette.as_deref()) {
        fields.push(format!("P: {}", value));
    }
    if let Some(value) = rendered_fps {
        fields.push(format!("FPS: {}", value));
    }

    fields.join(" | ")
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn fit_descriptor_text(
    descriptor: &OverlayDescriptor,
    rendered_fps: Option<u32>,
    normal_font: &sdl2::ttf::Font<'_, '_>,
    fps_font: &sdl2::ttf::Font<'_, '_>,
    maximum_width: u32,
) -> Result<String, String> {
    let full = format_descriptor(descriptor, rendered_fps);

    if segmented_text_fits(
        normal_font,
        fps_font,
        &full,
        rendered_fps,
        maximum_width,
    )? {
        return Ok(full);
    }

    let Some(shader) = descriptor.shader.as_deref() else {
        return truncate_plain_text(&full, normal_font, maximum_width);
    };

    let path = Path::new(shader);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value))
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(shader);
    let characters = stem.chars().collect::<Vec<_>>();

    for keep in (0..=characters.len()).rev() {
        let shortened = format!(
            "{}...{}",
            characters[..keep].iter().collect::<String>(),
            extension
        );
        let candidate = OverlayDescriptor {
            shader: Some(shortened),
            texture: descriptor.texture.clone(),
            palette: descriptor.palette.clone(),
        };
        let candidate_text = format_descriptor(&candidate, rendered_fps);

        if segmented_text_fits(
            normal_font,
            fps_font,
            &candidate_text,
            rendered_fps,
            maximum_width,
        )? {
            return Ok(candidate_text);
        }
    }

    truncate_plain_text(&full, normal_font, maximum_width)
}

fn split_fps_segment(text: &str, rendered_fps: Option<u32>) -> (&str, &str) {
    let Some(rendered_fps) = rendered_fps else {
        return (text, "");
    };

    let suffix = format!("FPS: {}", rendered_fps);
    let Some(prefix) = text.strip_suffix(&suffix) else {
        return (text, "");
    };

    (prefix, &text[prefix.len()..])
}

fn segmented_text_fits(
    normal_font: &sdl2::ttf::Font<'_, '_>,
    fps_font: &sdl2::ttf::Font<'_, '_>,
    text: &str,
    rendered_fps: Option<u32>,
    maximum_width: u32,
) -> Result<bool, String> {
    let (prefix, fps) = split_fps_segment(text, rendered_fps);
    let prefix_width = if prefix.is_empty() {
        0
    } else {
        normal_font
            .size_of(prefix)
            .map_err(|error| format!("Unable to measure subtitle text: {}", error))?
            .0
    };
    let fps_width = if fps.is_empty() {
        0
    } else {
        fps_font
            .size_of(fps)
            .map_err(|error| format!("Unable to measure FPS subtitle text: {}", error))?
            .0
    };

    Ok(prefix_width.saturating_add(fps_width) <= maximum_width)
}

fn truncate_plain_text(
    text: &str,
    font: &sdl2::ttf::Font<'_, '_>,
    maximum_width: u32,
) -> Result<String, String> {
    let characters = text.chars().collect::<Vec<_>>();

    for keep in (0..=characters.len()).rev() {
        let candidate = format!(
            "{}...",
            characters[..keep].iter().collect::<String>()
        );

        if text_fits(font, &candidate, maximum_width)? {
            return Ok(candidate);
        }
    }

    Ok("...".to_string())
}

fn text_fits(
    font: &sdl2::ttf::Font<'_, '_>,
    text: &str,
    maximum_width: u32,
) -> Result<bool, String> {
    let (width, _) = font
        .size_of(text)
        .map_err(|error| format!("Unable to measure subtitle text: {}", error))?;
    Ok(width <= maximum_width)
}

fn locate_subtitle_font() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("SCREENSHAVER_SUBTITLE_FONT") {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Ok(output) = Command::new("fc-match")
        .args(["-f", "%{file}\n", "DejaVu Sans:style=Book"])
        .output()
    {
        if output.status.success() {
            if let Some(value) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
            {
                let path = PathBuf::from(value);
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
    }

    for value in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/local/share/fonts/DejaVuSans.ttf",
    ] {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(
        "Unable to locate DejaVu Sans; set SCREENSHAVER_SUBTITLE_FONT to a sans-serif TTF file"
            .to_string(),
    )
}

fn draw_capsule_background(pixels: &mut [u8], width: u32, height: u32) {
    let radius = height as f32 / 2.0;
    let left_center = radius;
    let right_center = width as f32 - radius;
    let center_y = radius;

    for y in 0..height {
        for x in 0..width {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let inside = if px < left_center {
                distance_squared(px, py, left_center, center_y) <= radius * radius
            } else if px > right_center {
                distance_squared(px, py, right_center, center_y) <= radius * radius
            } else {
                true
            };

            if inside {
                let index = ((y as usize * width as usize) + x as usize) * 4;
                pixels[index..index + 4].copy_from_slice(&BACKGROUND_RGBA);
            }
        }
    }
}

fn distance_squared(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}

fn composite_surface(
    destination: &mut [u8],
    destination_width: u32,
    destination_height: u32,
    surface: &sdl2::surface::Surface<'_>,
    offset_x: u32,
    offset_y: u32,
) -> Result<(), String> {
    let source = surface
        .without_lock()
        .ok_or_else(|| "Unable to access rendered subtitle pixels".to_string())?;
    let source_pitch = usize::try_from(surface.pitch())
        .map_err(|_| "Subtitle surface pitch cannot be represented as usize".to_string())?;

    composite_text(
        destination,
        destination_width,
        destination_height,
        source,
        source_pitch,
        surface.width(),
        surface.height(),
        offset_x,
        offset_y,
    );

    Ok(())
}

fn composite_text(
    destination: &mut [u8],
    destination_width: u32,
    destination_height: u32,
    source: &[u8],
    source_pitch: usize,
    source_width: u32,
    source_height: u32,
    offset_x: u32,
    offset_y: u32,
) {
    for y in 0..source_height {
        for x in 0..source_width {
            let destination_x = offset_x + x;
            let destination_y = offset_y + y;

            if destination_x >= destination_width || destination_y >= destination_height {
                continue;
            }

            let source_index = y as usize * source_pitch + x as usize * 4;
            let destination_index =
                ((destination_y as usize * destination_width as usize) + destination_x as usize) * 4;

            let alpha = source[source_index + 3] as f32 / 255.0;
            let inverse = 1.0 - alpha;

            for channel in 0..3 {
                destination[destination_index + channel] = (
                    source[source_index + channel] as f32 * alpha
                        + destination[destination_index + channel] as f32 * inverse
                )
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }

            let destination_alpha = destination[destination_index + 3] as f32 / 255.0;
            destination[destination_index + 3] =
                ((alpha + destination_alpha * inverse) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
        }
    }
}

