#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub enum WallpaperMonitorMode {

    Mirror,
}


impl WallpaperMonitorMode {

    pub fn parse(
        value: &str,
    ) -> Result<Self, String> {

        match value
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "mirror" => {
                Ok(
                    Self::Mirror
                )
            }

            other => {
                Err(
                    format!(
                        "Unsupported wallpaper monitor_mode '{}'; supported values: mirror",
                        other,
                    )
                )
            }
        }
    }


    pub fn name(
        self,
    ) -> &'static str {

        match self {

            Self::Mirror => {
                "mirror"
            }
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
pub struct WallpaperSettings {

    pub monitor_mode:
        WallpaperMonitorMode,

    pub notifications:
        bool,
}

