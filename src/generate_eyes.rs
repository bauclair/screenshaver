//! Eyes texture generation and reusable animation-source preparation.
//!
//! The Eyes engine embeds three identically registered transparent PNG states:
//!
//! - `eye-open.png`
//! - `eye-half.png`
//! - `eye-closed.png`
//!
//! The source images are validated, cropped with one shared alpha bounding box,
//! resized identically, and arranged into the same tightly packed staggered
//! lattice used by the successful static Eyes checkpoint.
//!
//! `generate()` still returns the normal static all-open `GeneratedTexture`.
//! In addition, `build_animation_source()` exposes the already-prepared eye
//! frames, lattice instances, clean palette background, and reconstruction
//! helpers needed by `blink_eyes.rs` / `manage_textures.rs` in the next phase.
//!
//! A reconstructed animated frame always begins from a fresh palette-colored
//! background and composites exactly one mutually-exclusive state per eye.
//! Transparent PNG pixels therefore cannot reveal a previous eyelid state.

use image::imageops::{
    self,
    FilterType,
};

use image::{
    ImageFormat,
    RgbaImage,
};

use crate::generate_textures::{
    GeneratedTexture,
    TextureFamily,
    TEXTURE_SIZE,
};

use crate::palettes::PaletteColor;


// ============================================================
// Embedded source assets
// ============================================================

const EYE_OPEN_PNG:
    &[u8] =
    include_bytes!(
        "../assets/textures/eye-open.png"
    );

const EYE_HALF_PNG:
    &[u8] =
    include_bytes!(
        "../assets/textures/eye-half.png"
    );

const EYE_CLOSED_PNG:
    &[u8] =
    include_bytes!(
        "../assets/textures/eye-closed.png"
    );


// ============================================================
// Layout parameters
// ============================================================

const HORIZONTAL_PITCH_FACTOR:
    f32 =
    0.92;

const VERTICAL_PITCH_FACTOR:
    f32 =
    0.72;

const SOURCE_ALPHA_CROP_THRESHOLD:
    u8 =
    2;

const LATTICE_MARGIN:
    i32 =
    3;

pub const MIN_EYE_COUNT:
    usize =
    2;

pub const MAX_EYE_COUNT:
    usize =
    crate::define_constants::MAX_TEXTURE_PRIMITIVES;


// ============================================================
// Prepared animation source
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub(crate) enum EyeArtworkState {
    Open,
    Half,
    Closed,
}


#[derive(
    Debug,
    Clone,
    Copy,
)]
pub(crate) struct EyeInstance {
    left:
        i32,

    top:
        i32,
}


#[derive(Clone)]
pub(crate) struct EyesAnimationSource {

    open:
        RgbaImage,

    half:
        RgbaImage,

    closed:
        RgbaImage,

    instances:
        Vec<EyeInstance>,

    background:
        [u8; 4],

    requested_eye_count:
        usize,

    palette:
        PaletteColor,

    seed:
        u64,
}


impl EyesAnimationSource {

    pub(crate) fn eye_instance_count(
        &self,
    ) -> usize {

        self.instances.len()
    }


    pub(crate) fn requested_eye_count(
        &self,
    ) -> usize {

        self.requested_eye_count
    }


    pub(crate) fn palette(
        &self,
    ) -> PaletteColor {

        self.palette
    }


    pub(crate) fn seed(
        &self,
    ) -> u64 {

        self.seed
    }


    pub(crate) fn render_all_open_pixels(
        &self,
    ) -> Result<Vec<u8>, String> {

        self.render_with_state_provider(
            |_| {
                EyeArtworkState::Open
            }
        )
    }


    /// Rebuild a complete 1024x1024 Eyes texture from mutually-exclusive
    /// per-eye artwork states.
    ///
    /// The length must exactly match `eye_instance_count()`.  Every call starts
    /// from a clean palette-colored background, so transparent portions of a
    /// new state can never expose pixels belonging to an older state.
    pub(crate) fn render_states(
        &self,
        states: &[EyeArtworkState],
    ) -> Result<Vec<u8>, String> {

        if states.len()
            != self.instances.len()
        {
            return Err(
                format!(
                    "Eyes animation state count mismatch: expected {}, received {}",
                    self.instances.len(),
                    states.len(),
                )
            );
        }


        self.render_with_state_provider(
            |index| {
                states[index]
            }
        )
    }


    fn render_with_state_provider<F>(
        &self,
        mut state_for_eye: F,
    ) -> Result<Vec<u8>, String>
    where
        F: FnMut(usize) -> EyeArtworkState,
    {

        let mut pixels =
            new_background_buffer(
                self.background
            )?;


        for (
            eye_index,
            instance,
        ) in
            self.instances
                .iter()
                .enumerate()
        {
            let artwork =
                match state_for_eye(
                    eye_index
                ) {
                    EyeArtworkState::Open => {
                        &self.open
                    }

                    EyeArtworkState::Half => {
                        &self.half
                    }

                    EyeArtworkState::Closed => {
                        &self.closed
                    }
                };


            stamp_eye(
                &mut pixels,
                artwork,
                instance.left,
                instance.top,
            );
        }


        Ok(
            pixels
        )
    }
}


// ============================================================
// Source-frame set
// ============================================================

struct EyeFrames {
    open:
        RgbaImage,

    half:
        RgbaImage,

    closed:
        RgbaImage,
}


impl EyeFrames {

    fn load()
        -> Result<Self, String>
    {
        let open =
            decode_eye_frame(
                EYE_OPEN_PNG,
                "open",
            )?;

        let half =
            decode_eye_frame(
                EYE_HALF_PNG,
                "half-closed",
            )?;

        let closed =
            decode_eye_frame(
                EYE_CLOSED_PNG,
                "closed",
            )?;


        validate_matching_dimensions(
            &open,
            &half,
            &closed,
        )?;


        let crop =
            common_alpha_crop(
                [
                    &open,
                    &half,
                    &closed,
                ]
            )?;


        Ok(
            Self {
                open:
                    crop_frame(
                        &open,
                        crop,
                    ),

                half:
                    crop_frame(
                        &half,
                        crop,
                    ),

                closed:
                    crop_frame(
                        &closed,
                        crop,
                    ),
            }
        )
    }
}


#[derive(
    Debug,
    Clone,
    Copy,
)]
struct CropBounds {
    x:
        u32,

    y:
        u32,

    width:
        u32,

    height:
        u32,
}


// ============================================================
// Eye layout
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
)]
struct EyeLayout {
    eye_width:
        u32,

    eye_height:
        u32,

    horizontal_pitch:
        f32,

    vertical_pitch:
        f32,
}


impl EyeLayout {

    fn new(
        source_width: u32,
        source_height: u32,
        requested_primitive_count: usize,
    ) -> Result<Self, String> {

        if source_width == 0
            || source_height == 0
        {
            return Err(
                "Eye source image has zero width or height"
                    .to_string()
            );
        }


        let requested =
            requested_primitive_count
                .clamp(
                    MIN_EYE_COUNT,
                    MAX_EYE_COUNT,
                ) as f32;


        let source_aspect =
            source_width as f32
                / source_height as f32;


        let texture_area =
            TEXTURE_SIZE as f32
                * TEXTURE_SIZE as f32;


        let denominator =
            requested
                * source_aspect
                * HORIZONTAL_PITCH_FACTOR
                * VERTICAL_PITCH_FACTOR;


        let eye_height =
            (
                texture_area
                    / denominator.max(
                        f32::EPSILON
                    )
            )
            .sqrt()
            .round()
            .clamp(
                1.0,
                TEXTURE_SIZE as f32,
            )
            as u32;


        let eye_width =
            (
                eye_height as f32
                    * source_aspect
            )
            .round()
            .clamp(
                1.0,
                TEXTURE_SIZE as f32,
            )
            as u32;


        let horizontal_pitch =
            (
                eye_width as f32
                    * HORIZONTAL_PITCH_FACTOR
            )
            .max(
                1.0
            );


        let vertical_pitch =
            (
                eye_height as f32
                    * VERTICAL_PITCH_FACTOR
            )
            .max(
                1.0
            );


        Ok(
            Self {
                eye_width,
                eye_height,
                horizontal_pitch,
                vertical_pitch,
            }
        )
    }
}


// ============================================================
// Public generation API
// ============================================================

pub fn generate(
    palette: PaletteColor,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<GeneratedTexture, String> {

    let animation_source =
        build_animation_source(
            palette,
            seed,
            requested_primitive_count,
        )?;


    let pixels =
        animation_source
            .render_all_open_pixels()?;


    GeneratedTexture::new(
        TEXTURE_SIZE,
        TEXTURE_SIZE,
        pixels,
        TextureFamily::Eyes,
        palette,
        seed,
    )
}


/// Prepare everything required to reconstruct Eyes animation frames without
/// repeating PNG decoding, cropping, resizing, or lattice calculations.
pub(crate) fn build_animation_source(
    palette: PaletteColor,
    seed: u64,
    requested_primitive_count: usize,
) -> Result<EyesAnimationSource, String> {

    let frames =
        EyeFrames::load()?;


    let layout =
        EyeLayout::new(
            frames.open.width(),
            frames.open.height(),
            requested_primitive_count,
        )?;


    //---------------------------------------------------------
    // All three states are resized with exactly the same target
    // dimensions. Their shared source crop guarantees registration.
    //---------------------------------------------------------

    let open =
        preserve_eye_line_art(
            &imageops::resize(
                &frames.open,
                layout.eye_width,
                layout.eye_height,
                FilterType::Lanczos3,
            )
        );


    let half =
        preserve_eye_line_art(
            &imageops::resize(
                &frames.half,
                layout.eye_width,
                layout.eye_height,
                FilterType::Lanczos3,
            )
        );


    let closed =
        preserve_eye_line_art(
            &imageops::resize(
                &frames.closed,
                layout.eye_width,
                layout.eye_height,
                FilterType::Lanczos3,
            )
        );


    let instances =
        build_running_bond_instances(
            &open,
            &layout,
        );


    Ok(
        EyesAnimationSource {
            open,
            half,
            closed,
            instances,
            background:
                palette.map_rgba(
                    1.0
                ),
            requested_eye_count:
                requested_primitive_count
                    .clamp(
                        MIN_EYE_COUNT,
                        MAX_EYE_COUNT,
                    ),
            palette,
            seed,
        }
    )
}


// ============================================================
// Source preparation
// ============================================================

fn decode_eye_frame(
    bytes: &[u8],
    frame_name: &str,
) -> Result<RgbaImage, String> {

    image::load_from_memory_with_format(
        bytes,
        ImageFormat::Png,
    )
    .map_err(
        |error| {
            format!(
                "Unable to decode embedded {} eye PNG: {}",
                frame_name,
                error,
            )
        }
    )
    .map(
        |image| {
            image.to_rgba8()
        }
    )
}


fn validate_matching_dimensions(
    open: &RgbaImage,
    half: &RgbaImage,
    closed: &RgbaImage,
) -> Result<(), String> {

    let expected =
        (
            open.width(),
            open.height(),
        );


    for (
        frame_name,
        frame,
    ) in
        [
            (
                "half-closed",
                half,
            ),
            (
                "closed",
                closed,
            ),
        ]
    {
        let actual =
            (
                frame.width(),
                frame.height(),
            );


        if actual != expected {
            return Err(
                format!(
                    "Embedded {} eye PNG is {}x{}; expected {}x{} to match eye-open.png",
                    frame_name,
                    actual.0,
                    actual.1,
                    expected.0,
                    expected.1,
                )
            );
        }
    }


    Ok(())
}


fn common_alpha_crop(
    frames: [&RgbaImage; 3],
) -> Result<CropBounds, String> {

    let width =
        frames[0].width();

    let height =
        frames[0].height();


    let mut minimum_x =
        width;

    let mut minimum_y =
        height;

    let mut maximum_x =
        0_u32;

    let mut maximum_y =
        0_u32;

    let mut found_visible_pixel =
        false;


    for frame in
        frames
    {
        for (
            x,
            y,
            pixel,
        ) in
            frame.enumerate_pixels()
        {
            if pixel[3]
                <= SOURCE_ALPHA_CROP_THRESHOLD
            {
                continue;
            }


            found_visible_pixel =
                true;

            minimum_x =
                minimum_x.min(
                    x
                );

            minimum_y =
                minimum_y.min(
                    y
                );

            maximum_x =
                maximum_x.max(
                    x
                );

            maximum_y =
                maximum_y.max(
                    y
                );
        }
    }


    if !found_visible_pixel {
        return Err(
            "Embedded eye PNGs contain no visible pixels"
                .to_string()
        );
    }


    Ok(
        CropBounds {
            x:
                minimum_x,

            y:
                minimum_y,

            width:
                maximum_x
                    - minimum_x
                    + 1,

            height:
                maximum_y
                    - minimum_y
                    + 1,
        }
    )
}


fn crop_frame(
    source: &RgbaImage,
    crop: CropBounds,
) -> RgbaImage {

    imageops::crop_imm(
        source,
        crop.x,
        crop.y,
        crop.width,
        crop.height,
    )
    .to_image()
}


// ============================================================
// Eye artwork preparation
// ============================================================

fn preserve_eye_line_art(
    source: &RgbaImage,
) -> RgbaImage {

    source.clone()
}


// ============================================================
// Staggered placement
// ============================================================

fn build_running_bond_instances(
    eye: &RgbaImage,
    layout: &EyeLayout,
) -> Vec<EyeInstance> {

    let texture_size =
        TEXTURE_SIZE as f32;


    let maximum_rows =
        (
            texture_size
                / layout.vertical_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    let maximum_columns =
        (
            texture_size
                / layout.horizontal_pitch
        )
        .ceil() as i32
        + LATTICE_MARGIN;


    let mut instances =
        Vec::new();


    //---------------------------------------------------------
    // Preserve the exact iteration order of the previous static
    // implementation so all-open reconstruction remains identical.
    //---------------------------------------------------------

    for row in
        -maximum_rows
            ..=
        maximum_rows
    {
        let row_offset =
            if row
                & 1
                == 0
            {
                0.0
            } else {
                layout.horizontal_pitch
                    * 0.5
            };


        let center_y =
            texture_size
                * 0.5
            + row as f32
                * layout.vertical_pitch;


        for column in
            -maximum_columns
                ..=
            maximum_columns
        {
            let center_x =
                texture_size
                    * 0.5
                + column as f32
                    * layout.horizontal_pitch
                + row_offset;


            let left =
                (
                    center_x
                        - eye.width() as f32
                            * 0.5
                )
                .round() as i32;


            let top =
                (
                    center_y
                        - eye.height() as f32
                            * 0.5
                )
                .round() as i32;


            //-------------------------------------------------
            // Do not retain lattice instances that are completely
            // outside the 1024x1024 texture.  They are useful while
            // conceptually constructing an infinite running-bond
            // lattice, but they never contribute a pixel to the
            // generated texture.
            //
            // This distinction becomes important for animation:
            // BlinkController schedules by EyeInstance index.  If
            // fully offscreen instances are retained, an allowed
            // blink slot can be consumed by an eye the user can
            // never see, making animation appear inactive.
            //-------------------------------------------------

            let right =
                left
                    + eye.width() as i32;

            let bottom =
                top
                    + eye.height() as i32;


            let intersects_texture =
                right > 0
                    && bottom > 0
                    && left < TEXTURE_SIZE as i32
                    && top < TEXTURE_SIZE as i32;


            if intersects_texture {
                instances.push(
                    EyeInstance {
                        left,
                        top,
                    }
                );
            }
        }
    }


    instances
}


// ============================================================
// Pixel-buffer reconstruction
// ============================================================

fn new_background_buffer(
    background: [u8; 4],
) -> Result<Vec<u8>, String> {

    let pixel_count =
        TEXTURE_SIZE as usize
            * TEXTURE_SIZE as usize;


    let byte_count =
        pixel_count
            .checked_mul(
                4
            )
            .ok_or_else(
                || {
                    "Eye texture buffer size overflow"
                        .to_string()
                }
            )?;


    let mut pixels =
        Vec::with_capacity(
            byte_count
        );


    for _ in
        0..pixel_count
    {
        pixels.extend_from_slice(
            &background
        );
    }


    Ok(
        pixels
    )
}


fn stamp_eye(
    destination: &mut [u8],
    eye: &RgbaImage,
    destination_left: i32,
    destination_top: i32,
) {

    for source_y in
        0..eye.height()
    {
        let destination_y =
            destination_top
                + source_y as i32;


        if destination_y < 0
            || destination_y
                >= TEXTURE_SIZE as i32
        {
            continue;
        }


        for source_x in
            0..eye.width()
        {
            let destination_x =
                destination_left
                    + source_x as i32;


            if destination_x < 0
                || destination_x
                    >= TEXTURE_SIZE as i32
            {
                continue;
            }


            let source_pixel =
                eye.get_pixel(
                    source_x,
                    source_y,
                );


            let source_alpha =
                source_pixel[3] as u32;


            if source_alpha == 0 {
                continue;
            }


            let destination_index =
                (
                    destination_y as usize
                        * TEXTURE_SIZE as usize
                    + destination_x as usize
                )
                    * 4;


            let inverse_alpha =
                255_u32
                    - source_alpha;


            for channel in
                0..3
            {
                let source_value =
                    source_pixel[
                        channel
                    ] as u32;


                let destination_value =
                    destination[
                        destination_index
                            + channel
                    ] as u32;


                destination[
                    destination_index
                        + channel
                ] =
                    (
                        source_value
                            * source_alpha
                        + destination_value
                            * inverse_alpha
                        + 127
                    )
                        .div_euclid(
                            255
                        )
                        .min(
                            255
                        ) as u8;
            }


            destination[
                destination_index
                    + 3
            ] =
                255;
        }
    }
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn embedded_eye_frames_decode_and_match() {

        let frames =
            EyeFrames::load()
                .expect(
                    "embedded eye frames"
                );


        assert!(
            frames.open.width()
                > 0
        );

        assert!(
            frames.open.height()
                > 0
        );

        assert_eq!(
            frames.open.dimensions(),
            frames.half.dimensions()
        );

        assert_eq!(
            frames.open.dimensions(),
            frames.closed.dimensions()
        );
    }


    #[test]
    fn requested_count_is_clamped_to_eye_range() {

        let frames =
            EyeFrames::load()
                .expect(
                    "embedded eye frames"
                );


        let minimum =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                0,
            )
            .expect(
                "minimum eye layout"
            );


        let explicit_minimum =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                MIN_EYE_COUNT,
            )
            .expect(
                "explicit minimum eye layout"
            );


        assert_eq!(
            minimum.eye_width,
            explicit_minimum.eye_width
        );

        assert_eq!(
            minimum.eye_height,
            explicit_minimum.eye_height
        );


        let maximum =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                usize::MAX,
            )
            .expect(
                "maximum eye layout"
            );


        let explicit_maximum =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                MAX_EYE_COUNT,
            )
            .expect(
                "explicit maximum eye layout"
            );


        assert_eq!(
            maximum.eye_width,
            explicit_maximum.eye_width
        );

        assert_eq!(
            maximum.eye_height,
            explicit_maximum.eye_height
        );
    }


    #[test]
    fn denser_request_produces_smaller_eyes() {

        let frames =
            EyeFrames::load()
                .expect(
                    "embedded eye frames"
                );


        let sparse =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                2,
            )
            .expect(
                "sparse eye layout"
            );


        let dense =
            EyeLayout::new(
                frames.open.width(),
                frames.open.height(),
                1024,
            )
            .expect(
                "dense eye layout"
            );


        assert!(
            dense.eye_width
                < sparse.eye_width
        );

        assert!(
            dense.eye_height
                < sparse.eye_height
        );
    }


    #[test]
    fn animation_source_renders_all_open_texture() {

        let source =
            build_animation_source(
                PaletteColor::new(
                    180,
                    180,
                    180,
                ),
                12345,
                64,
            )
            .expect(
                "Eyes animation source"
            );


        let pixels =
            source
                .render_all_open_pixels()
                .expect(
                    "all-open Eyes pixels"
                );


        assert_eq!(
            pixels.len(),
            TEXTURE_SIZE as usize
                * TEXTURE_SIZE as usize
                * 4
        );


        assert!(
            source.eye_instance_count()
                > 0
        );
    }


    #[test]
    fn render_states_rejects_wrong_state_count() {

        let source =
            build_animation_source(
                PaletteColor::new(
                    180,
                    180,
                    180,
                ),
                12345,
                64,
            )
            .expect(
                "Eyes animation source"
            );


        assert!(
            source
                .render_states(
                    &[
                        EyeArtworkState::Open
                    ]
                )
                .is_err()
        );
    }


    #[test]
    fn animation_source_contains_only_visible_instances() {

        let source =
            build_animation_source(
                PaletteColor::new(
                    180,
                    180,
                    180,
                ),
                12345,
                16,
            )
            .expect(
                "Eyes animation source"
            );


        assert!(
            !source.instances.is_empty()
        );


        for instance in
            &source.instances
        {
            let right =
                instance.left
                    + source.open.width() as i32;

            let bottom =
                instance.top
                    + source.open.height() as i32;


            assert!(
                right > 0
                    && bottom > 0
                    && instance.left
                        < TEXTURE_SIZE as i32
                    && instance.top
                        < TEXTURE_SIZE as i32
            );
        }
    }


    #[test]
    fn generator_returns_standard_texture() {

        let texture =
            generate(
                PaletteColor::new(
                    180,
                    180,
                    180,
                ),
                12345,
                64,
            )
            .expect(
                "eye generation"
            );


        assert_eq!(
            texture.specification.family,
            TextureFamily::Eyes
        );


        assert_eq!(
            texture.width,
            TEXTURE_SIZE
        );


        assert_eq!(
            texture.height,
            TEXTURE_SIZE
        );


        assert!(
            texture
                .validate_standard()
                .is_ok()
        );
    }
}
