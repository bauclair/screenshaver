use std::time::Duration;

pub struct ParsedDuration {
    pub duration: Duration,
    pub diagnostics: Vec<String>,
}

pub fn parse_duration(input: &str) -> ParsedDuration {

    let mut diagnostics = Vec::new();

    diagnostics.push(format!(
        "[PARSE_DURATION] raw input = {}",
        input
    ));

    let trimmed = input.trim();

    // =========================================================
    // HOURS (h)
    // =========================================================

    if let Some(v) = trimmed.strip_suffix('h') {

        match v.parse::<u64>() {

            Ok(n) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] parsed hours = {}",
                    n
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(n * 60 * 60),
                    diagnostics,
                };
            }

            Err(_) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] invalid hours '{}', defaulting to 60s",
                    v
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(60),
                    diagnostics,
                };
            }
        }
    }

    // =========================================================
    // MINUTES (m)
    // =========================================================

    if let Some(v) = trimmed.strip_suffix('m') {

        match v.parse::<u64>() {

            Ok(n) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] parsed minutes = {}",
                    n
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(n * 60),
                    diagnostics,
                };
            }

            Err(_) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] invalid minutes '{}', defaulting to 60s",
                    v
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(60),
                    diagnostics,
                };
            }
        }
    }

    // =========================================================
    // SECONDS (s)
    // =========================================================

    if let Some(v) = trimmed.strip_suffix('s') {

        match v.parse::<u64>() {

            Ok(n) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] parsed seconds = {}",
                    n
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(n),
                    diagnostics,
                };
            }

            Err(_) => {
                diagnostics.push(format!(
                    "[PARSE_DURATION] invalid seconds '{}', defaulting to 60s",
                    v
                ));

                return ParsedDuration {
                    duration: Duration::from_secs(60),
                    diagnostics,
                };
            }
        }
    }

    // =========================================================
    // RAW fallback (e.g. "120")
    // =========================================================

    match trimmed.parse::<u64>() {

        Ok(n) => {
            diagnostics.push(format!(
                "[PARSE_DURATION] parsed raw seconds = {}",
                n
            ));

            ParsedDuration {
                duration: Duration::from_secs(n),
                diagnostics,
            }
        }

        Err(_) => {
            diagnostics.push(format!(
                "[PARSE_DURATION] invalid input '{}', defaulting to 60s",
                trimmed
            ));

            ParsedDuration {
                duration: Duration::from_secs(60),
                diagnostics,
            }
        }
    }
}