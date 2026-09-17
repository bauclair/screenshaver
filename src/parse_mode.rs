//
// ------------------------------------------------------------
// Mode Types
// ------------------------------------------------------------
//

#[derive(Debug)]
pub enum ModeType {

    Random,

    Ordered,

    Playlist,

    Single,

    Invalid,
}

//
// ------------------------------------------------------------
// Parsed Result
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct ParsedMode {

    pub mode: ModeType,

    pub argument: String,

    pub diagnostics: Vec<String>,
}

//
// ------------------------------------------------------------
// Parse operation.mode
// ------------------------------------------------------------
//

pub fn parse_mode(input: &str) -> ParsedMode {

    let mut diagnostics = Vec::new();

    diagnostics.push(format!(
        "[PARSE_MODE] raw input = {}",
        input
    ));

    let pieces: Vec<&str> = input.split(':').collect();

    let (mode, argument) =
        match pieces.as_slice() {

            ["random", interval] =>
                (ModeType::Random, (*interval).to_string()),

            ["ordered", interval] =>
                (ModeType::Ordered, (*interval).to_string()),

            ["single", selector] =>
                (ModeType::Single, (*selector).to_string()),

            ["playlist", playlist_id, _interval] =>
                (ModeType::Playlist, (*playlist_id).to_string()),

            _ => {
                diagnostics.push(
                    "[PARSE_MODE] Invalid format".to_string()
                );

                return ParsedMode {
                    mode: ModeType::Invalid,
                    argument: String::new(),
                    diagnostics,
                };
            }
        };

    diagnostics.push(format!(
        "[PARSE_MODE] mode = {:?}",
        mode
    ));

    diagnostics.push(format!(
        "[PARSE_MODE] argument = {}",
        argument
    ));

    ParsedMode {
        mode,
        argument,
        diagnostics,
    }
}
