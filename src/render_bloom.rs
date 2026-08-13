//! Bloom post-processing definitions and renderer support.
//!
//! Checkpoint 1 intentionally defines only the Bloom data model. OpenGL bloom
//! extraction, blur, and composition will be added in later checkpoints.

pub(crate) const BLOOM_INTENSITY_MIN: f32 =
    0.0;

pub(crate) const BLOOM_INTENSITY_MAX: f32 =
    2.0;

pub(crate) const BLOOM_INTENSITY_DEFAULT: f32 =
    1.0;


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
)]
pub(crate) enum BloomMode {

    #[default]
    Off,

    Highlight,
}


impl BloomMode {

    pub(crate) fn parse(
        value: &str,
    ) -> Result<Self, String> {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "off" => {
                Ok(
                    Self::Off
                )
            }

            "highlight" => {
                Ok(
                    Self::Highlight
                )
            }

            other => {
                Err(
                    format!(
                        "Unsupported bloom mode '{}'; supported values: off, highlight",
                        other,
                    )
                )
            }
        }
    }


    pub(crate) fn name(
        self,
    ) -> &'static str {

        match self {
            Self::Off => "off",
            Self::Highlight => "highlight",
        }
    }


    #[allow(dead_code)]
    pub(crate) fn is_enabled(
        self,
    ) -> bool {

        !matches!(
            self,
            Self::Off
        )
    }
}


pub(crate) fn validate_bloom_intensity(
    value: f32,
) -> Result<f32, String> {

    if value.is_finite()
        && (BLOOM_INTENSITY_MIN
            ..=BLOOM_INTENSITY_MAX)
            .contains(
                &value
            )
    {
        return Ok(
            value
        );
    }


    Err(
        format!(
            "Bloom intensity {} is outside the supported range {:.2}-{:.2}",
            value,
            BLOOM_INTENSITY_MIN,
            BLOOM_INTENSITY_MAX,
        )
    )
}
