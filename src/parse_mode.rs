//
// ------------------------------------------------------------
// Mode Types
// ------------------------------------------------------------
//

#[derive(Debug)]
pub enum ModeType {

    Random,

    Ordered,

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

    if pieces.len() != 2 {

        diagnostics.push(
            "[PARSE_MODE] Invalid format".to_string()
        );

        return ParsedMode {

            mode: ModeType::Invalid,

            argument: String::new(),

            diagnostics,
        };
    }

    let mode = match pieces[0] {

        "random" => ModeType::Random,

        "ordered" => ModeType::Ordered,

        "single" => ModeType::Single,

        _ => ModeType::Invalid,
    };

    diagnostics.push(format!(
        "[PARSE_MODE] mode = {:?}",
        mode
    ));

    diagnostics.push(format!(
        "[PARSE_MODE] argument = {}",
        pieces[1]
    ));

    ParsedMode {

        mode,

        argument: pieces[1].to_string(),

        diagnostics,
    }
}