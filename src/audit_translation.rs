//! Read-only developer audit for Screenshaver localization coverage.
//!
//! This intentionally uses lexical source scanning rather than Rust compiler internals so the
//! audit remains lightweight and database-independent. It distinguishes direct presentation
//! defects from runtime human-readable English text. All non-suppressed runtime prose
//! defects and true catalog defects make the audit fail.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_LOCALE: &str = "en-US";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Actionable,
    Review,
}

#[derive(Debug, Clone)]
struct Finding {
    severity: Severity,
    path: PathBuf,
    line: usize,
    text: String,
}

pub fn run(requested_locale: Option<&str>, requested_module: Option<&str>) -> Result<bool, String> {
    let root = project_root()?;
    let src = root.join("src");
    let locale = match requested_locale {
        Some(value) => value.trim().to_string(),
        None => configured_locale().unwrap_or_else(|| DEFAULT_LOCALE.to_string()),
    };

    let keys_path = src.join("localization_catalog/keys.rs");
    let locale_path = locale_module_path(&src, &locale)?;
    let english = parse_key_catalog(&keys_path)?;
    let translated = if locale.eq_ignore_ascii_case(DEFAULT_LOCALE) {
        english.keys().cloned().collect::<BTreeSet<_>>()
    } else {
        parse_translation_catalog(&locale_path)?
    };

    let missing = english
        .keys()
        .filter(|key| !translated.contains(*key) && !locale_invariant_key(key))
        .cloned()
        .collect::<Vec<_>>();

    let files = match requested_module {
        Some(module) => vec![resolve_module_path(&src, module)?],
        None => {
            let mut files = Vec::new();
            collect_rs_files(&src, &mut files)?;
            files.sort();
            files
        }
    };

    let english_values = english.values().cloned().collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    let mut candidate_count = 0usize;
    let mut suppressed_count = 0usize;

    for path in &files {
        if path == &keys_path || path == &locale_path || path.ends_with("audit_translation.rs") {
            continue;
        }
        let text = fs::read_to_string(path)
            .map_err(|e| format!("Unable to read '{}': {}", path.display(), e))?;
        scan_file(
            path,
            &text,
            &english_values,
            &mut candidate_count,
            &mut suppressed_count,
            &mut findings,
        );
    }

    println!("[TRANSLATION AUDIT] Locale: {}", locale);
    println!("[TRANSLATION AUDIT] Source root: {}", src.display());
    match requested_module {
        Some(_) => println!("[TRANSLATION AUDIT] Source scope: {}", files[0].display()),
        None => println!("[TRANSLATION AUDIT] Source scope: entire src/ tree"),
    }

    print_findings("DEFINITE LOCALIZATION DEFECTS", &findings, Severity::Actionable);
    print_findings("NON-ACTIONABLE REVIEW FINDINGS", &findings, Severity::Review);

    println!("\nMISSING {} TRANSLATIONS", locale);
    if missing.is_empty() {
        println!("    none");
    } else {
        for key in &missing {
            println!("    {}", key);
        }
    }

    let actionable_source = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Actionable)
        .count();
    let review = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Review)
        .count();
    let actionable = actionable_source + missing.len();

    println!("\nSUMMARY");
    println!("    Rust files scanned:        {}", files.len());
    println!("    Candidate strings:         {}", candidate_count);
    println!("    Suppressed invariants:     {}", suppressed_count);
    println!("    Definite source defects:   {}", actionable_source);
    println!("    Non-actionable review:     {}", review);
    println!("    Missing translations:      {}", missing.len());
    println!("    Actionable findings:       {}", actionable);
    println!("\n[TRANSLATION AUDIT] {}", if actionable == 0 { "PASS" } else { "FAIL" });
    Ok(actionable == 0)
}

fn project_root() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("SCREENSHAVER_SOURCE_ROOT") {
        let p = PathBuf::from(dir);
        if is_project_root(&p) {
            return Ok(p);
        }
        return Err(format!(
            "SCREENSHAVER_SOURCE_ROOT '{}' is not a Screenshaver source root (Cargo.toml and src/ are required).",
            p.display()
        ));
    }

    let current = std::env::current_dir()
        .map_err(|error| format!("Unable to determine current working directory: {}", error))?;

    for candidate in current.ancestors() {
        if is_project_root(candidate) {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(format!(
        "Screenshaver source tree not found from current directory '{}'. Run --audit-translation from the project root (or one of its subdirectories), or set SCREENSHAVER_SOURCE_ROOT.",
        current.display()
    ))
}

fn is_project_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("src").is_dir()
}

fn resolve_module_path(src: &Path, requested: &str) -> Result<PathBuf, String> {
    let requested = requested.trim();
    if requested.is_empty() || requested.starts_with('-') {
        return Err("--audit-translation MODULE must name a Rust source module".to_string());
    }

    let module = requested.strip_suffix(".rs").unwrap_or(requested);
    if module.contains('/') || module.contains('\\') || module == "." || module == ".." {
        return Err(format!(
            "--audit-translation MODULE must be a module name such as 'import_data' or 'import_data.rs', not a path: '{}'",
            requested
        ));
    }

    let direct = src.join(format!("{}.rs", module));
    if direct.is_file() {
        return Ok(direct);
    }

    let mut matches = Vec::new();
    collect_named_rs_files(src, &format!("{}.rs", module), &mut matches)?;
    matches.sort();

    match matches.len() {
        0 => Err(format!(
            "Rust module '{}' was not found under '{}'.",
            requested,
            src.display()
        )),
        1 => Ok(matches.remove(0)),
        _ => Err(format!(
            "Rust module '{}' is ambiguous; matching files: {}",
            requested,
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn collect_named_rs_files(dir: &Path, filename: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("Unable to read '{}': {}", dir.display(), error))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            collect_named_rs_files(&path, filename, out)?;
        } else if path.file_name().and_then(|value| value.to_str()) == Some(filename) {
            out.push(path);
        }
    }
    Ok(())
}

fn configured_locale() -> Option<String> {
    let path = crate::locate_paths::config_path();
    let text = fs::read_to_string(path).ok()?;
    let mut in_language = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_language = line == "[language]";
            continue;
        }
        if in_language && line.starts_with("locale") {
            let value = line.split_once('=')?.1.trim().trim_matches('"').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn locale_module_path(src: &Path, locale: &str) -> Result<PathBuf, String> {
    if locale.eq_ignore_ascii_case(DEFAULT_LOCALE) {
        return Ok(src.join("localization_catalog/keys.rs"));
    }
    let module = locale.to_ascii_lowercase().replace('-', "_");
    let path = src.join("localization_catalog").join(format!("{}.rs", module));
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "No source translation catalog exists for locale '{}' at '{}'.",
            locale,
            path.display()
        ))
    }
}

fn parse_key_catalog(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Unable to read '{}': {}", path.display(), e))?;
    let mut out = BTreeMap::new();
    let mut key: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(v) = field_string(t, "key:") {
            key = Some(v);
        }
        if let Some(v) = field_string(t, "english_text:") {
            if let Some(k) = key.take() {
                out.insert(k, v);
            }
        }
    }
    if out.is_empty() {
        Err(format!("No translation keys parsed from '{}'.", path.display()))
    } else {
        Ok(out)
    }
}

fn parse_translation_catalog(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("Unable to read '{}': {}", path.display(), e))?;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        if let Some(v) = field_string(line.trim(), "key:") {
            out.insert(v);
        }
    }
    if out.is_empty() {
        Err(format!("No translations parsed from '{}'.", path.display()))
    } else {
        Ok(out)
    }
}

fn field_string(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.trim();
    let rest = rest.strip_prefix('"')?;
    let end = rest.rfind('"')?;
    Some(rest[..end].replace("\\\"", "\"").replace("\\n", "\n"))
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Unable to read '{}': {}", dir.display(), e))?
    {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().and_then(|v| v.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn scan_file(
    path: &Path,
    text: &str,
    english: &BTreeSet<String>,
    candidate_count: &mut usize,
    suppressed_count: &mut usize,
    findings: &mut Vec<Finding>,
) {
    let mut in_block_comment = false;
    let lines = text.lines().collect::<Vec<_>>();

    for (idx, raw) in lines.iter().enumerate() {
        let mut line = raw.trim();
        if in_block_comment {
            if let Some(position) = line.find("*/") {
                line = &line[position + 2..];
                in_block_comment = false;
            } else {
                continue;
            }
        }
        if line.starts_with("/*") {
            in_block_comment = true;
            continue;
        }
        if line.starts_with("//") {
            continue;
        }
        if contains_localization_call(line) {
            continue;
        }

        for literal in string_literals(line) {
            if !looks_human(&literal) {
                continue;
            }
            *candidate_count += 1;

            if english.contains(&literal) || intentionally_invariant(&literal, line, path) {
                *suppressed_count += 1;
                continue;
            }

            findings.push(Finding {
                severity: Severity::Actionable,
                path: path.to_path_buf(),
                line: idx + 1,
                text: literal,
            });
        }
    }
}

fn contains_localization_call(line: &str) -> bool {
    line.contains("runtime_text(")
        || line.contains("runtime_text_with_params(")
        || line.contains("tr(")
        || line.contains("trp(")
}

fn source_context(lines: &[&str], index: usize, radius: usize) -> String {
    let start = index.saturating_sub(radius);
    let end = (index + radius + 1).min(lines.len());
    lines[start..end].join("\n")
}

fn direct_presentation_context(context: &str) -> bool {
    // Known egui and Screenshaver presentation sinks. Looking a couple of lines around the
    // literal catches common multi-line RichText/Label/format! constructions without trying to
    // perform full Rust data-flow analysis.
    const SINKS: &[&str] = &[
        "ui.label(",
        "ui.heading(",
        "ui.button(",
        "ui.small_button(",
        "ui.checkbox(",
        "ui.radio(",
        "ui.radio_value(",
        "ui.selectable_label(",
        "ui.selectable_value(",
        "ui.hyperlink_to(",
        "ui.add(egui::Label",
        "egui::Label::new(",
        "egui::RichText::new(",
        ".on_hover_text(",
        ".on_hover_ui(",
        "summary(ui,",
        "content_section(ui,",
        "InspectionCheck",
        "result.pass(",
        "result.fail(",
        "MessageDialog",
        "show_simple_message_box",
        "show_message_box",
    ];

    SINKS.iter().any(|sink| context.contains(sink))
}

fn string_literals(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            i += 1;
            let mut escaped = false;
            while i < bytes.len() {
                if escaped {
                    escaped = false;
                    i += 1;
                    continue;
                }
                if bytes[i] == b'\\' {
                    escaped = true;
                    i += 1;
                    continue;
                }
                if bytes[i] == b'"' {
                    out.push(line[start..i].to_string());
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn looks_human(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || !t.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    t.contains(' ')
        || t.contains("{}")
        || t.contains("{error}")
        || t.ends_with('.')
        || t.ends_with(':')
        || t.ends_with('!')
        || t.ends_with('?')
}

fn intentionally_invariant(s: &str, line: &str, path: &Path) -> bool {
    let t = s.trim();

    if line.contains("include_str!") || line.contains("include_bytes!") {
        return true;
    }

    // SQL/SQLite statements and fragments are executable machine text, not presentation.
    let upper = t.to_ascii_uppercase();
    if [
        "SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE ", "ALTER ", "DROP ",
        "PRAGMA ", "VACUUM ", "BEGIN ", "COMMIT", "ROLLBACK", "WITH ",
    ]
    .iter()
    .any(|prefix| upper.starts_with(prefix))
        || line.contains("query_row(")
        || line.contains("prepare(") && upper.contains("SELECT")
    {
        return true;
    }

    // Generated QML/GLSL and similar embedded source is code even when its tokens contain spaces.
    if path.file_name().and_then(|value| value.to_str())
        .map(|name| name.starts_with("construct_lock_screen_"))
        .unwrap_or(false)
        && (t.starts_with("import Qt")
            || t.contains("property ")
            || t.contains("anchors.")
            || t.contains("function ")
            || t.contains("event.")
            || t.contains("Qt.")
            || t.contains("Math."))
    {
        return true;
    }

    if t.starts_with('[') && t.ends_with(']') {
        return true;
    }
    if t.starts_with("http://") || t.starts_with("https://") {
        return true;
    }
    if t.contains(".glsl")
        || t.contains(".fs")
        || t.contains(".json")
        || t.contains(".db")
        || t.contains(".toml")
        || t.contains(".qml")
    {
        return true;
    }

    if matches!(
        t,
        "screensaver"
            | "wallpaper"
            | "unassigned"
            | "specific"
            | "random"
            | "inherit_target"
            | "explicit"
            | "sha256"
            | "screenshaver-package-v1"
            | "policy shader_export_id"
            | "membership playlist_export_id"
            | "membership policy_export_id"
            | "membership position"
    ) {
        return true;
    }

    false
}

fn locale_invariant_key(key: &str) -> bool {
    // Product/brand identity is intentionally identical in every locale and therefore does not
    // require a redundant locale-specific row merely to satisfy the audit.
    matches!(key, "app.name")
}

fn print_findings(title: &str, findings: &[Finding], severity: Severity) {
    println!("\n{}", title);
    let selected = findings
        .iter()
        .filter(|finding| finding.severity == severity)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        println!("    none");
        return;
    }
    for finding in selected {
        println!(
            "{}:{}\n    {:?}",
            finding.path.display(),
            finding.line,
            finding.text
        );
    }
}
