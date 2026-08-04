#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub(crate) enum ColorPrecisionPolicy {
    Auto,
    Standard,
    High,
}

impl ColorPrecisionPolicy {
    pub(crate) fn name(
        self,
    ) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Standard => "standard",
            Self::High => "high",
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub(crate) enum RenderTargetPrecision {
    Standard,
    High,
}

impl RenderTargetPrecision {
    pub(crate) fn name(
        self,
    ) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::High => "high",
        }
    }

    pub(crate) fn internal_format(
        self,
    ) -> i32 {
        match self {
            Self::Standard => {
                gl::RGBA8 as i32
            }

            Self::High => {
                gl::RGBA16F as i32
            }
        }
    }

    pub(crate) fn external_format(
        self,
    ) -> u32 {
        gl::RGBA
    }

    pub(crate) fn pixel_type(
        self,
    ) -> u32 {
        match self {
            Self::Standard => {
                gl::UNSIGNED_BYTE
            }

            Self::High => {
                gl::HALF_FLOAT
            }
        }
    }

    pub(crate) fn internal_format_name(
        self,
    ) -> &'static str {
        match self {
            Self::Standard => "RGBA8",
            Self::High => "RGBA16F",
        }
    }

    pub(crate) fn bytes_per_pixel(
        self,
    ) -> usize {
        match self {
            Self::Standard => 4,
            Self::High => 8,
        }
    }
}

#[derive(
    Debug,
    Clone,
)]
pub(crate) struct RenderPrecisionSelection {
    pub requested:
        ColorPrecisionPolicy,

    pub selected:
        RenderTargetPrecision,

    pub fell_back:
        bool,

    pub fallback_reason:
        Option<String>,
}


impl RenderPrecisionSelection {
    pub(crate) fn direct(
        requested: ColorPrecisionPolicy,
        selected: RenderTargetPrecision,
    ) -> Self {
        Self {
            requested,
            selected,
            fell_back:
                false,
            fallback_reason:
                None,
        }
    }


    pub(crate) fn fallback(
        requested: ColorPrecisionPolicy,
        selected: RenderTargetPrecision,
        reason: String,
    ) -> Self {
        Self {
            requested,
            selected,
            fell_back:
                true,
            fallback_reason:
                Some(reason),
        }
    }
}

