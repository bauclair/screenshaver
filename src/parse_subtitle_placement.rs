#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleVerticalPosition {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleHorizontalPosition {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubtitlePlacement {
    pub vertical: SubtitleVerticalPosition,
    pub horizontal: SubtitleHorizontalPosition,
}

impl Default for SubtitlePlacement {
    fn default() -> Self {
        Self {
            vertical: SubtitleVerticalPosition::Bottom,
            horizontal: SubtitleHorizontalPosition::Left,
        }
    }
}

impl SubtitlePlacement {
    pub fn name(self) -> &'static str {
        match (self.vertical, self.horizontal) {
            (SubtitleVerticalPosition::Top, SubtitleHorizontalPosition::Left) => "top:left",
            (SubtitleVerticalPosition::Top, SubtitleHorizontalPosition::Center) => "top:center",
            (SubtitleVerticalPosition::Top, SubtitleHorizontalPosition::Right) => "top:right",
            (SubtitleVerticalPosition::Bottom, SubtitleHorizontalPosition::Left) => "bottom:left",
            (SubtitleVerticalPosition::Bottom, SubtitleHorizontalPosition::Center) => "bottom:center",
            (SubtitleVerticalPosition::Bottom, SubtitleHorizontalPosition::Right) => "bottom:right",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedSubtitlePlacement {
    pub placement: SubtitlePlacement,
    pub warning: Option<String>,
}

pub fn parse(value: Option<&str>) -> ParsedSubtitlePlacement {
    let Some(value) = value else {
        return ParsedSubtitlePlacement {
            placement: SubtitlePlacement::default(),
            warning: None,
        };
    };

    let normalized = value.trim().to_ascii_lowercase();

    let placement = match normalized.as_str() {
        "top:left" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Top,
            horizontal: SubtitleHorizontalPosition::Left,
        }),
        "top:center" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Top,
            horizontal: SubtitleHorizontalPosition::Center,
        }),
        "top:right" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Top,
            horizontal: SubtitleHorizontalPosition::Right,
        }),
        "bottom:left" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Bottom,
            horizontal: SubtitleHorizontalPosition::Left,
        }),
        "bottom:center" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Bottom,
            horizontal: SubtitleHorizontalPosition::Center,
        }),
        "bottom:right" => Some(SubtitlePlacement {
            vertical: SubtitleVerticalPosition::Bottom,
            horizontal: SubtitleHorizontalPosition::Right,
        }),
        _ => None,
    };

    match placement {
        Some(placement) => ParsedSubtitlePlacement {
            placement,
            warning: None,
        },
        None => ParsedSubtitlePlacement {
            placement: SubtitlePlacement::default(),
            warning: Some(format!(
                "[CONFIG] Invalid subtitle_placement \"{}\"; using bottom:left",
                value
            )),
        },
    }
}

