//! Read-only developer audit for Screenshaver localization coverage.
//!
//! This intentionally uses lexical source scanning rather than Rust compiler internals so the
//! audit remains lightweight and database-independent. It distinguishes direct presentation
//! defects from runtime human-readable English text. All non-suppressed runtime prose
//! defects and true catalog defects make the audit fail.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
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

    let source_inventory = source_inventory(&src)?;
    let active_graph = active_rust_files(&root, &src)?;
    let active_files = source_inventory
        .iter()
        .filter(|path| active_graph.contains(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let orphaned_files = source_inventory
        .iter()
        .filter(|path| !active_files.contains(*path))
        .cloned()
        .collect::<Vec<_>>();

    let (files, requested_module_is_orphaned) = match requested_module {
        Some(module) => {
            let path = resolve_module_path(&src, module)?;
            let orphaned = !active_files.contains(&path);
            (vec![path], orphaned)
        }
        None => {
            let mut files = active_files.iter().cloned().collect::<Vec<_>>();
            files.sort();
            (files, false)
        }
    };

    let english_values = english.values().cloned().collect::<BTreeSet<_>>();
    let mut findings = Vec::new();
    let mut candidate_count = 0usize;
    let mut suppressed_count = 0usize;

    for path in &files {
        if requested_module_is_orphaned {
            continue;
        }
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
        None => println!("[TRANSLATION AUDIT] Source scope: active Rust module graph"),
    }

    if requested_module_is_orphaned {
        println!("\nMODULE STATUS");
        println!("    Inactive/orphaned Rust source.");
        println!("    No active module reference was found.");
        println!("    Localization audit skipped.");
    } else {
        print_findings("DEFINITE LOCALIZATION DEFECTS", &findings, Severity::Actionable);
        print_findings("NON-ACTIONABLE REVIEW FINDINGS", &findings, Severity::Review);
    }

    if requested_module.is_none() {
        println!("\nORPHANED RUST SOURCE FILES");
        if orphaned_files.is_empty() {
            println!("    none");
        } else {
            for path in &orphaned_files {
                println!("    {}", display_relative(&root, path));
            }
        }
    }

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
    let scanned_files = if requested_module_is_orphaned { 0 } else { files.len() };
    println!("    Active Rust files scanned: {}", scanned_files);
    if requested_module.is_none() {
        println!("    Orphaned Rust files:       {}", orphaned_files.len());
    }
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


fn source_inventory(src: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_rs_files(src, &mut files)?;
    files.retain(|path| !is_intentional_source_copy(path));
    files.sort();
    files.dedup();
    Ok(files)
}

fn active_rust_files(root: &Path, src: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let mut active = BTreeSet::new();
    let mut queue = VecDeque::new();

    // The main Screenshaver crate and the separate KDE renderer crate both contribute to
    // reachability. KDE files themselves remain outside the localization scan; following that
    // crate merely prevents shared main-tree files from being misclassified as orphaned.
    for candidate in [
        src.join("main.rs"),
        src.join("lib.rs"),
        root.join("kde-renderer/src/lib.rs"),
        root.join("kde-renderer/src/main.rs"),
    ] {
        if candidate.is_file() {
            queue.push_back(candidate);
        }
    }

    while let Some(path) = queue.pop_front() {
        if !active.insert(path.clone()) {
            continue;
        }

        let text = fs::read_to_string(&path)
            .map_err(|error| format!("Unable to read '{}': {}", path.display(), error))?;
        for child in referenced_rust_modules(&path, &text) {
            if child.is_file() && !active.contains(&child) {
                queue.push_back(child);
            }
        }
    }

    Ok(active)
}

fn referenced_rust_modules(parent: &Path, text: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let parent_dir = match parent.parent() {
        Some(value) => value,
        None => return out,
    };

    let mut pending_path: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();

        if line.starts_with("//") {
            continue;
        }

        if line.starts_with("#[path") {
            pending_path = attribute_path_value(line);
            continue;
        }

        if let Some(module) = module_declaration_name(line) {
            if let Some(relative) = pending_path.take() {
                out.push(normalize_path(parent_dir.join(relative)));
            } else {
                let module_dir = rust_module_directory(parent);
                let sibling = module_dir.join(format!("{}.rs", module));
                let nested = module_dir.join(&module).join("mod.rs");
                if sibling.is_file() {
                    out.push(normalize_path(sibling));
                } else if nested.is_file() {
                    out.push(normalize_path(nested));
                }
            }
        } else if !line.is_empty() && !line.starts_with("#[") {
            pending_path = None;
        }
    }

    out
}

fn rust_module_directory(parent: &Path) -> PathBuf {
    let parent_dir = parent.parent().unwrap_or_else(|| Path::new(""));
    match parent.file_name().and_then(|value| value.to_str()) {
        Some("main.rs") | Some("lib.rs") | Some("mod.rs") => parent_dir.to_path_buf(),
        Some(filename) if filename.ends_with(".rs") => {
            let stem = filename.trim_end_matches(".rs");
            parent_dir.join(stem)
        }
        _ => parent_dir.to_path_buf(),
    }
}

fn attribute_path_value(line: &str) -> Option<String> {
    let first_quote = line.find('"')?;
    let rest = &line[first_quote + 1..];
    let second_quote = rest.find('"')?;
    Some(rest[..second_quote].to_string())
}

fn module_declaration_name(line: &str) -> Option<String> {
    let line = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub(self) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);

    let rest = line.strip_prefix("mod ")?.trim();
    if !rest.ends_with(';') {
        return None;
    }

    let name = rest.trim_end_matches(';').trim();
    if !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Some(name.to_string())
    } else {
        None
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn is_intentional_source_copy(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.to_ascii_lowercase().contains("copy"))
        .unwrap_or(false)
}

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
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

            let catalog_literal =
                literal
                    .replace("\\n", "\n");

            let matches_catalog =
                english.contains(&literal)
                    || english.contains(&catalog_literal);

            if (matches_catalog && !is_direct_presentation_literal(line))
                || intentionally_invariant(&literal, line, path)
            {
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

fn is_direct_presentation_literal(line: &str) -> bool {
    // A raw literal passed directly to a UI presentation API is still a localization
    // defect even when identical English text exists somewhere in the canonical catalog.
    // Catalog membership proves that text is translatable; it does not prove this source
    // location actually performs a translation lookup.
    [
        "Button::new(",
        ".button(",
        "ui.label(",
        "ui.heading(",
        "RichText::new(",
        ".selected_text(",
        ".on_hover_text(",
        ".on_hover_ui(",
        "selectable_label(",
        "checkbox(",
        "radio_value(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
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

    // Portable export serialization, archive layout, filename conventions, and the
    // interchange-format identifier are machine-facing compatibility contracts.
    // Keep these exemptions scoped to export_data.rs so identical raw English text
    // elsewhere remains subject to normal localization auditing.
    let is_export_data = path
        .file_name()
        .and_then(|value| value.to_str())
        == Some("export_data.rs");

    if is_export_data
        && matches!(
            t,
            "{}\\t{}\\t{}\\n"
                | "{}\\t{}\\t{}\\t{}\\n"
                | "backup/managed-shaders/{}"
                | "{}.zip"
                | "Screenshaver-Export-{}.zip"
                | "Screenshaver Export Format 1"
        )
    {
        return true;
    }

    // compile_shader.rs is shared by the normal Screenshaver executable and the
    // standalone KDE renderer library. This exact message reports an internal programming
    // error if the process-global formatter hook is registered more than once; it is not
    // presentation text. The shared module's normal English fallback messages are not
    // exempted here: because they are canonical catalog text, scan_file() suppresses them
    // through the catalog comparison above.
    let is_compile_shader = path
        .file_name()
        .and_then(|value| value.to_str())
        == Some("compile_shader.rs");

    if is_compile_shader
        && t == "compile-shader text formatter is already initialized"
    {
        return true;
    }


    // analyze_audio.rs emits stable developer/runtime telemetry for FFT and audio
    // analysis. These tagged lines report numerical analyzer state rather than localized
    // presentation text. Keep the exemption scoped to this module so [AUDIO] or
    // [FFT-MOTION-DIAG] prose elsewhere remains subject to normal localization auditing.
    let is_analyze_audio = path
        .file_name()
        .and_then(|value| value.to_str())
        == Some("analyze_audio.rs");

    if is_analyze_audio
        && (t.starts_with("[AUDIO]")
            || t.starts_with("[FFT-MOTION-DIAG]"))
    {
        return true;
    }


    // edit_shader.rs uses bracketed subsystem tags exclusively for developer/runtime
    // diagnostics written through Screenshaver logging. They are intentionally stable
    // diagnostic text, not localized presentation. The remaining exact strings below
    // are internal state encodings, assertion text, or technical shader identifiers.
    let is_edit_shader = path
        .file_name()
        .and_then(|value| value.to_str())
        == Some("edit_shader.rs");

    if is_edit_shader
        && (t.starts_with("[EDIT_SHADER]")
            || t.starts_with("[TEXTURE]")
            || t.starts_with("[AUDIO_MOTION]")
            || t.starts_with("[CONFIG]")
            || matches!(
                t,
                "shader_paths was checked for emptiness"
                    | "Native GLSL"
                    | "existing policy"
                    | "resolved defaults"
                    | "rejected:"
                    | "unavailable:"
                    | "rejected: {}"
                    | "unavailable: {}"
                    | "single:{}"
                    | "playlist:{}:{}"
            ))
    {
        return true;
    }

    // These modules contain technical/internal diagnostics or stable environment
    // classification labels rather than localized presentation text.
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    if filename == "display_lock_authentication.rs"
        && t == "current user"
    {
        return true;
    }

    if filename == "generate_clouds.rs"
        && t == "Cloud texture buffer size overflow"
    {
        return true;
    }

    // define_wallpaper.rs is shared with the standalone KDE renderer crate, which
    // intentionally does not depend on the application's localization manager. This
    // exact message validates the stable monitor_mode/mirror configuration tokens.
    if filename == "define_wallpaper.rs"
        && t == "Unsupported wallpaper monitor_mode '{}'; supported values: mirror"
    {
        return true;
    }

    if filename == "parse_subtitle_placement.rs"
        && t.starts_with("[CONFIG] Invalid subtitle_placement")
    {
        return true;
    }

    if filename == "detect_desktop_environment.rs"
        && matches!(t, "KDE Plasma" | "GNOME" | "XFCE" | "Other" | "Unknown")
    {
        return true;
    }

    // Procedural texture generators return internal allocation diagnostics and contain
    // test-only assertion labels. None of these strings are presentation text. Keep every
    // exemption scoped by module and exact literal so future prose in these modules is still
    // audited normally.
    let procedural_texture_invariant = match filename {
        "generate_bricks.rs" => matches!(
            t,
            "Brick texture buffer size overflow"
                | "Different seeds produced identical brick walls"
                | "brick generation"
        ),
        "generate_cellular.rs" => matches!(
            t,
            "Cellular texture buffer size overflow"
                | "cellular generation"
        ),
        "generate_facets.rs" => matches!(
            t,
            "Facet texture buffer size overflow"
                | "facet texture generation"
        ),
        "generate_hexagons.rs" => matches!(
            t,
            "Hexagon texture buffer size overflow"
                | "hexagon texture generation"
        ),
        "generate_marble.rs" => matches!(
            t,
            "Marble texture buffer size overflow"
                | "marble generation"
        ),
        "generate_mesh.rs" => matches!(
            t,
            "Mesh texture buffer size overflow"
                | "Different seeds produced identical values across every sampled coordinate"
                | "mesh generation"
        ),
        "generate_noise.rs" => matches!(
            t,
            "Noise texture buffer size overflow"
                | "Different seeds produced identical television snow"
                | "noise generation"
        ),
        "generate_radial.rs" => matches!(
            t,
            "Radial texture buffer size overflow"
                | "Different seeds produced identical values across every sampled coordinate"
                | "radial generation"
        ),
        "generate_scales.rs" => matches!(
            t,
            "Scale texture buffer size overflow"
                | "scale generation"
        ),
        _ => false,
    };

    if procedural_texture_invariant {
        return true;
    }

    // Observation-only lock presentation messages are structured runtime telemetry. Backend
    // names and field identifiers are intentionally stable for log comparison and diagnostics.
    if filename == "monitor_lock_presentation.rs"
        && t.starts_with("[LOCK] ")
    {
        return true;
    }

    // This value is an egui widget identity, not text rendered to the user.
    if filename == "nested_tabs.rs"
        && t == "nested_config_disabled_grid_{}"
    {
        return true;
    }

    // ISF parser failures describe source-format parsing, thumbnail failures describe internal
    // image construction, and session-query errors describe backend plumbing. These exact
    // technical errors are intentionally stable and are not Control Center presentation text.
    if filename == "parse_isf.rs"
        && matches!(
            t,
            "Unterminated block comment while searching for ISF metadata"
                | "No valid ISF JSON metadata block was found"
        )
    {
        return true;
    }

    if filename == "preview_texture_thumbnail.rs"
        && matches!(
            t,
            "Texture thumbnail size must be greater than zero"
                | "Unable to construct {}x{} RGBA image for texture thumbnail"
        )
    {
        return true;
    }

    if filename == "query_session.rs"
        && matches!(
            t,
            "Backend unavailable: {}"
                | "Session query failed: {}"
        )
    {
        return true;
    }

    // Post-processing renderer errors are OpenGL construction/uniform diagnostics. These
    // modules may participate in renderer paths where adding localization dependencies would
    // be undesirable; keep the exemptions exact and module-scoped.
    let renderer_invariant = match filename {
        "render_dithering.rs" => matches!(
            t,
            "Unable to build dithering presentation program: {}"
                | "OpenGL failed to allocate the dithering vertex array"
                | "Dithering presentation program is missing a required uniform"
        ),
        "render_fxaa.rs" => matches!(
            t,
            "Unable to build FXAA presentation program: {}"
                | "OpenGL failed to allocate the FXAA vertex array"
                | "FXAA presentation program is missing a required uniform"
        ),
        "render_passthrough.rs" => matches!(
            t,
            "Unable to build passthrough presentation program: {}"
                | "OpenGL failed to allocate the passthrough vertex array"
                | "Passthrough presentation program does not expose the uScene sampler"
        ),
        _ => false,
    };

    if renderer_invariant {
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
