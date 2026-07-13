//
// ------------------------------------------------------------
// Parsed result
// ------------------------------------------------------------
//

#[derive(Debug)]
pub struct ParsedInterval {

    pub seconds: u64,

    pub diagnostics: Vec<String>,
}

//
// ------------------------------------------------------------
// Interval parsing
// ------------------------------------------------------------
//

pub fn parse_interval(input: &str) -> ParsedInterval {

    let mut diagnostics = Vec::new();

    diagnostics.push(format!(
        "[PARSE_INTERVAL] raw input = {}",
        input
    ));

    let trimmed = input.trim();

    //---------------------------------------------------------
    // Case 1: "mode:value" format
    //---------------------------------------------------------

    let parts: Vec<&str> = trimmed.split(':').collect();

    let raw_value = if parts.len() == 2 {
        diagnostics.push(format!(
            "[PARSE_INTERVAL] mode prefix = {}",
            parts[0]
        ));

        parts[1]
    } else {
        trimmed
    };

    //---------------------------------------------------------
    // Case 2: numeric parsing
    //---------------------------------------------------------

    let seconds = match raw_value.parse::<u64>() {

        Ok(v) => {
            diagnostics.push(format!(
                "[PARSE_INTERVAL] parsed seconds = {}",
                v
            ));
            v
        }

        Err(_) => {

            diagnostics.push(format!(
                "[PARSE_INTERVAL] invalid number '{}', defaulting to 60",
                raw_value
            ));

            60
        }
    };

    ParsedInterval {

        seconds,

        diagnostics,
    }
}