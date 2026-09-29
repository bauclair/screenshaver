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

    // Batch v7: these modules contain backend, database, parser, and runtime-control
    // diagnostics rather than presentation text. Keep exemptions module-scoped and exact
    // (or restricted to a stable diagnostic prefix) so future UI prose remains auditable.
    if filename == "control_wallpaper.rs"
        && matches!(
            t,
            "Unable to request exclusive renderer ownership from the active wallpaper renderer: {}"
                | "Timed out after {} ms waiting for the active wallpaper renderer to acknowledge suspension; refusing to start a second renderer"
                | "Using XDG runtime directory: {}"
                | "XDG_RUNTIME_DIR does not name an existing directory: {}"
                | "XDG_RUNTIME_DIR unavailable; using fallback runtime directory: {}"
                | "Unable to locate a usable wallpaper control runtime directory; checked XDG_RUNTIME_DIR and {}"
                | "[WALLPAPER_CONTROL] {}"
        )
    {
        return true;
    }

    if filename == "create_wayland_lock_context.rs"
        && (t.starts_with("[LOCK TEST]")
            || matches!(
                t,
                "Cannot create an EGL lock context with a zero-sized surface"
                    | "libwayland-egl is unavailable"
                    | "Lock-surface width {} exceeds EGL limits"
                    | "Lock-surface height {} exceeds EGL limits"
                    | "Wayland display pointer is null; EGL interop is unavailable"
                    | "eglGetDisplay returned EGL_NO_DISPLAY"
                    | "eglInitialize failed: {:?}"
                    | "eglBindAPI(EGL_OPENGL_API) failed: {:?}"
                    | "eglChooseConfig failed: {:?}"
                    | "No EGL window configuration supports desktop OpenGL"
                    | "eglCreateContext(OpenGL 3.3 core) failed: {:?}"
                    | "Unable to create wl_egl_window: {}"
                    | "eglCreateWindowSurface failed: {:?}"
                    | "eglMakeCurrent failed: {:?}"
                    | "eglSwapBuffers failed: {:?}"
            ))
    {
        return true;
    }

    if filename == "database_factory.rs"
        && matches!(
            t,
            "Unable to read default shader '{}': {}"
                | "Default shader '{}' is not valid UTF-8: {}"
                | "Default shader '{}' was unexpectedly classified as ShaderToy"
                | "Default shader '{}' was unexpectedly classified as ISF"
                | "Default shader path '{}' has no valid UTF-8 filename"
                | "Default shader path '{}' has no parent directory"
                | "Unable to register default shader '{}': {}"
                | "Unable to begin localization-catalog synchronization transaction: {}"
                | "Unable to prepare language-catalog synchronization: {}"
                | "Unable to synchronize factory language '{}': {}"
                | "Unable to prepare translation-key synchronization: {}"
                | "Unable to synchronize factory translation key '{}': {}"
                | "Unable to prepare translation synchronization: {}"
                | "Unable to synchronize factory translation '{}:{}': {}"
                | "Unable to commit localization factory catalog synchronization: {}"
        )
    {
        return true;
    }

    if filename == "locate_wallpaper.rs"
        && matches!(
            t,
            "Unable to open database for wallpaper shader discovery: {}"
                | "Unable to prepare wallpaper shader discovery query: {}"
                | "Unable to query wallpaper shader discovery rows: {}"
                | "Unable to decode wallpaper shader discovery row: {}"
        )
    {
        return true;
    }

    if filename == "open_database.rs"
        && matches!(
            t,
            "Unable to open existing database '{}': {}"
                | "Unable to enable SQLite foreign-key enforcement: {}"
                | "Unable to configure SQLite synchronous mode: {}"
                | "Unable to configure SQLite busy timeout: {}"
        )
    {
        return true;
    }

    if filename == "parse_interval.rs"
        && t.starts_with("[PARSE_INTERVAL]")
    {
        return true;
    }

    if filename == "parse_mode.rs"
        && t.starts_with("[PARSE_MODE]")
    {
        return true;
    }

    // Language names here are factory catalog metadata. english_name intentionally names
    // every locale in English; native_name intentionally names it in that locale itself.
    let is_localization_catalog_mod = filename == "mod.rs"
        && path.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some("localization_catalog");

    if is_localization_catalog_mod
        && matches!(
            t,
            "English (United States)"
                | "Spanish (United States)"
                | "Español (Estados Unidos)"
        )
    {
        return true;
    }

    // Batch v9: audio capture, Xfce integration, and text-overlay implementation
    // diagnostics are technical runtime/configuration messages. The compact P/T/FPS overlay
    // prefixes are stable presentation notation; user/external descriptor values and lyrics
    // remain untouched. Every exemption below is module-scoped and exact. The generated Xfce
    // desktop-entry Comment is localized in source and is deliberately not exempted here.
    if filename == "mod.rs"
        && path.parent()
            .and_then(|parent| parent.file_name())
            .and_then(|value| value.to_str())
            == Some("audio_backend")
        && matches!(
            t,
            "[AUDIO] Unable to start demand-driven audio runtime worker: {}"
                | "[AUDIO] Demand-driven Audio Bloom capture active: {}"
                | "[AUDIO] Audio Bloom capture could not be started; continuing with zero audio bands: {}"
                | "[AUDIO] Demand-driven Audio Bloom capture inactive"
                | "[AUDIO] Attempting backend: PulseAudio"
                | "[AUDIO] Selected [PULSEAUDIO] backend"
                | "No compatible audio backend available"
                | "[AUDIO] {} backend unavailable: {}"
        )
    {
        return true;
    }

    if filename == "pulseaudio.rs"
        && matches!(
            t,
            "Unable to start PulseAudio worker thread: {}"
                | "Timed out while starting PulseAudio playback capture: {}"
                | "[AUDIO] PulseAudio capture worker stopped: {}"
                | "Unable to create PulseAudio main loop"
                | "Unable to create PulseAudio context"
                | "Unable to connect to PulseAudio-compatible server: {}"
                | "[AUDIO] Default playback sink: {}"
                | "[AUDIO] Playback monitor source: {}"
                | "PulseAudio capture sample specification is invalid"
                | "Screenshaver Audio Bloom Capture"
                | "Unable to create PulseAudio recording stream"
                | "[AUDIO] Requested capture fragment: {} bytes (~{:.1} ms)"
                | "Unable to connect recording stream to '{}': {}"
                | "[AUDIO] Actual capture fragment: {} bytes (~{:.1} ms)"
                | "[AUDIO] Actual capture fragment unavailable"
                | "[AUDIO] Playback capture active: {} Hz, {} channels, S16 native-endian PCM"
                | "Unable to report PulseAudio capture readiness: {}"
                | "PulseAudio context failed during capture: {}"
                | "PulseAudio context terminated during capture"
                | "PulseAudio recording stream failed during capture"
                | "PulseAudio recording stream terminated during capture"
                | "Unable to discard captured PulseAudio data: {}"
                | "Unable to discard PulseAudio capture hole: {}"
                | "Unable to read PulseAudio capture data: {}"
                | "[AUDIO] Capture diagnostic: {} bytes received in last {} s; {} bytes / {} fragments total"
                | "[AUDIO] PulseAudio playback capture stopped"
                | "PulseAudio context failed: {}"
                | "PulseAudio context terminated during initialization"
                | "Timed out while connecting to PulseAudio-compatible server"
                | "PulseAudio server did not report a default playback sink"
                | "default playback sink"
                | "Default PulseAudio sink has no monitor source"
                | "PulseAudio failed while querying the default sink"
                | "playback monitor source"
                | "Timed out while querying PulseAudio {}"
                | "PulseAudio recording stream failed during initialization"
                | "PulseAudio recording stream terminated during initialization"
                | "Timed out while starting PulseAudio recording stream"
                | "PulseAudio main loop quit unexpectedly: {:?}"
                | "PulseAudio main loop failed: {}"
        )
    {
        return true;
    }

    // SDL/OpenGL subtitle-overlay construction and shader diagnostics are renderer-internal technical text.
    if filename == "display_overlay.rs"
        && matches!(
            t,
            "Unable to create SDL subtitle texture: {}"
                | "Subtitle width cannot be represented as usize"
                | "Subtitle pitch overflow"
                | "Unable to upload subtitle pixels: {}"
                | "Unable to draw subtitle overlay: {}"
                | "OpenGL failed to allocate subtitle overlay resources"
                | "Unable to link subtitle overlay program: {}"
                | "Subtitle overlay shader source contains a null byte"
                | "Unable to compile subtitle overlay shader: {}"
                | "unknown shader compilation error"
                | "unknown program link error"
                | "static subtitle uniform name"
        )
    {
        return true;
    }

    // QBE database-access errors are data-layer diagnostics; qbe_layout.rs owns user-facing presentation.
    if filename == "query_database.rs"
        && matches!(
            t,
            "Unable to open database while loading QBE lookup values: {}"
                | "Unable to open database while loading QBE texture names: {}"
                | "Unable to open database while loading QBE palette choices: {}"
                | "Unable to open database while loading QBE Shader Type values: {}"
                | "Unable to open database while loading QBE Playlist Name values: {}"
                | "Unable to prepare QBE Playlist Name query: {}"
                | "Unable to query QBE Playlist Name choices: {}"
                | "Unable to decode QBE Playlist Name row: {}"
                | "Unable to prepare QBE texture-catalog query: {}"
                | "Unable to query QBE texture choices: {}"
                | "Unable to decode QBE texture-catalog row: {}"
                | "Unable to prepare QBE curated-palette query: {}"
                | "Unable to query QBE curated-palette choices: {}"
                | "Unable to decode QBE curated-palette row: {}"
                | "Unable to prepare QBE Shader Type query: {}"
                | "Unable to query QBE Shader Type choices: {}"
                | "Unable to decode QBE Shader Type row: {}"
                | "Unable to open database while executing Policy List QBE: {}"
                | "Unable to parse Policy List QBE: {}"
                | "Unable to open database while loading the complete Policy List: {}"
                | "Unable to count total shader policies for QBE: {}"
                | "Unable to prepare Policy List QBE statement: {}"
                | "Unable to execute Policy List QBE: {}"
                | "Unable to decode Policy List QBE row: {}"
                | "Total shader-policy count {} cannot be represented as usize"
                | "\\n             WHERE "
        )
    {
        return true;
    }

    // Xfce native-lock presentation findings are X11/GLX/process telemetry and backend diagnostics.
    if filename == "present_screen_lock_xfce.rs"
        && matches!(
            t,
            "{} is empty"
                | "Unable to parse {} value '{}': {}"
                | "{} contains an invalid zero X11 window ID"
                | "[LOCK] XFCE lock presentation window detected: 0x{:X}"
                | "[LOCK] XFCE OpenGL presentation: opening X11 display"
                | "Unable to connect to the X11 display while verifying XFCE presentation window 0x{:X}: {}"
                | "XGetWindowAttributes failed for XFCE presentation window 0x{:X}"
                | "XFCE presentation window 0x{:X} has invalid geometry {}x{}"
                | "[LOCK] XFCE lock presentation window verified: 0x{:X}, geometry={}x{}, depth={}, map_state={}"
                | "[LOCK] XFCE shader presentation: loading Screenshaver configuration"
                | "Unable to load Screenshaver configuration for XFCE lock presentation: {}"
                | "[LOCK] XFCE shader presentation: choosing GLX framebuffer configuration"
                | "Unable to choose GLX framebuffer configuration for XFCE lock presentation: {}"
                | "[LOCK] XFCE shader presentation: selected GLX visual 0x{:X}"
                | "Unable to create GLX context for XFCE lock presentation: {}"
                | "[LOCK] XFCE shader presentation: making context current on supplied window"
                | "Unable to make GLX context current on XFCE presentation window 0x{:X}: {}"
                | "[LOCK] XFCE shader presentation: GLX context is current"
                | "OpenGL symbol name contained an interior NUL"
                | "[LOCK] XFCE shader presentation: constructing FrameRenderEngine"
                | "Unable to construct FrameRenderEngine for XFCE lock presentation: {}"
                | "[LOCK] XFCE shader presentation initialized: window=0x{:X}, geometry={}x{}"
                | "[LOCK] XFCE shader presentation started: window=0x{:X}, geometry={}x{}"
                | "[LOCK] XFCE shader presentation frames displayed: {}"
                | "[LOCK] XFCE authentication-dialog process poll failed: {}"
                | "[LOCK] Unable to start XFCE authentication-dialog monitor thread; shader presentation will continue: {}"
                | "[LOCK] XFCE authentication dialog state: {}"
                | "Unable to read /proc while checking for {}: {}"
        )
    {
        return true;
    }

    // Current-schema reconstruction findings are migration/storage validation diagnostics, not presentation text.
    if filename == "write_current.rs"
        && matches!(
            t,
            "Refusing to reconstruct database because destination already exists: {}"
                | "Unable to create reconstruction database '{}': {}"
                | "Unable to create current database schema in '{}': {}"
                | "Unable to begin reconstruction transaction: {}"
                | "Unable to commit reconstructed current database: {}"
                | "{}; additionally unable to remove failed reconstruction database '{}': {}"
                | "Unable to begin factory-catalog reconstruction transaction: {}"
                | "Unable to prepare reconstructed texture-catalog insert: {}"
                | "Unable to reconstruct texture family '{}': {}"
                | "Unable to prepare reconstructed curated-palette insert: {}"
                | "Unable to reconstruct curated palette entry '{}': {}"
                | "Unable to commit reconstructed factory catalogs: {}"
                | "Unable to prepare reconstructed shader insert: {}"
                | "Unable to reconstruct shader '{}' from '{}': {}"
                | "MigrationData contains duplicate shader migration ID {}"
                | "MigrationData shader-ID mapping is incomplete: mapped {}, expected {}"
                | "Unable to prepare reconstructed policy insert: {}"
                | "Policy '{}' references unknown shader migration ID {}"
                | "Policy Name"
                | "Unable to reconstruct policy '{}': {}"
                | "MigrationData contains duplicate policy migration ID {}"
                | "Playlist Name"
                | "Unable to reconstruct playlist '{}': {}"
                | "MigrationData contains duplicate playlist migration ID {}"
                | "Playlist '{}' references unknown policy migration ID {}"
                | "Unable to reconstruct member {} of playlist '{}': {}"
                | "Unable to reconstruct application defaults: {}"
                | "{} idle-timeout value {} exceeds SQLite integer range"
                | "{} target default contains conflicting texture primitive counts: {} and {}"
                | "Unable to reconstruct {} target defaults: {}"
                | "{} runtime target references unknown policy migration ID {}"
                | "ordered interval"
                | "random interval"
                | "{} runtime target references unknown playlist migration ID {}"
                | "playlist interval"
                | "Unable to reconstruct {} runtime target: {}"
                | "Unable to write reconstructed schema metadata: {}"
                | "Unable to validate reconstructed {} row count: {}"
                | "Reconstructed {} row count mismatch: expected {}, found {}"
                | "{} must contain between 1 and 128 characters; found {}"
                | "{} produced an empty comparison key"
                | "{} value {} exceeds SQLite integer range"
        )
    {
        return true;
    }

    // xfconf-query emits this English array-header prefix as command output.
    // Screenshaver filters it structurally; it is not application-authored presentation text.
    if filename == "construct_lock_screen_xfce.rs"
        && t == "Value is an array with"
    {
        return true;
    }

    if filename == "construct_lock_screen_xfce.rs"
        && matches!(
            t,
            "XFCE lock-screen configuration did not verify successfully: {:?}"
                | "XFCE Screensaver is not available at {}"
                | "xfconf-query is not available at {}"
                | "HOME is not set; unable to locate the user's XFCE configuration"
                | "HOME does not contain an absolute path: {}"
                | "Unable to inspect XFCE screensaver properties: {}"
                | "Unable to inspect XFCE screensaver properties"
                | "Unable to query XFCE screensaver themes: {}"
                | "Unable to query XFCE screensaver themes"
                | "Value is an array with "
                | "Unable to migrate the legacy permanent Screenshaver XFCE theme selection: {}"
                | "Unable to migrate the legacy permanent Screenshaver XFCE theme selection"
                | "{}; command exited with status {}"
                | "Unable to determine parent directory for {}"
                | "Unable to create directory {}: {}"
                | "Unable to create temporary file {}: {}"
                | "Unable to write temporary file {}: {}"
                | "Unable to synchronize temporary file {}: {}"
                | "Unable to install user configuration file {}: {}"
        )
    {
        return true;
    }

    if filename == "construct_text_overlay.rs"
        && matches!(
            t,
            "Unable to initialize SDL_ttf: {}"
                | "Unable to load subtitle font '{}': {}"
                | "Unable to load bold subtitle font '{}': {}"
                | "Cannot construct a text overlay without content"
                | "Unable to render subtitle text: {}"
                | "Unable to convert subtitle text surface: {}"
                | "Unable to render FPS subtitle text: {}"
                | "Unable to convert FPS subtitle surface: {}"
                | "Subtitle overlay dimensions overflow"
                | "Cannot construct a message overlay without content"
                | "Unable to load message-overlay font '{}': {}"
                | "Unable to render message-overlay text: {}"
                | "Unable to convert message-overlay text surface: {}"
                | "Message overlay dimensions overflow"
                | "P: {}"
                | "T: {}"
                | "FPS: {}"
                | "Unable to measure subtitle text: {}"
                | "Unable to measure FPS subtitle text: {}"
                | "DejaVu Sans:style=Book"
                | "Unable to locate DejaVu Sans; set SCREENSHAVER_SUBTITLE_FONT to a sans-serif TTF file"
                | "DejaVu Sans Condensed:style=Bold"
                | "DejaVu Sans Condensed:style=Book"
                | "Unable to locate {}; set {} to the corresponding TTF file"
                | "DejaVu Sans Condensed Bold"
                | "DejaVu Sans Condensed"
                | "Unable to access rendered subtitle pixels"
                | "Subtitle surface pitch cannot be represented as usize"
                | "Cannot construct a lyrics panel for a zero-sized viewport"
                | "Unable to initialize SDL_ttf for lyrics: {}"
                | "Unable to load current-lyrics font '{}': {}"
                | "Unable to measure current lyrics text: {}"
                | "Unable to load lyrics font '{}': {}"
                | "Unable to render lyrics text: {}"
                | "Unable to convert lyrics text surface: {}"
                | "Lyrics panel dimensions overflow"
                | "Unable to access rendered lyrics pixels"
                | "Lyrics surface pitch cannot be represented as usize"
        )
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


    // Batch v8: backend/session, renderer, shader-widget, and procedural-texture
    // diagnostics. These are technical implementation messages or stable machine
    // tokens rather than localized presentation. Exemptions remain exact and
    // module-scoped so future human-facing prose in these modules is still audited.
    let batch_v8_invariant = match filename {
        "logind.rs" => matches!(
            t,
            "Unable to connect to system bus: {}"
                | "Unable to create logind manager proxy: {}"
                | "Unable to locate session: {}"
                | "Unable to create logind session proxy: {}"
                | "Unable to read logind IdleHint: {}"
        ),
        "x11.rs" => matches!(
            t,
            "[X11] Idle threshold = {} ms"
                | "XScreenSaver extension is unavailable"
                | "[X11] XScreenSaver extension available (event_base={}, error_base={})"
                | "Unable to allocate XScreenSaverInfo"
                | "[X11] XScreenSaverQueryInfo failed"
                | "XScreenSaverQueryInfo failed"
                | "[X11] poll_state: idle={} ms, timeout={} ms"
                | "[X11] State = Idle (idle={} ms, timeout={} ms)"
        ),
        "x11_connection.rs" => matches!(
            t,
            "[X11] Opening X11 display"
                | "Unable to open X11 display"
                | "[X11] Connected to {}"
                | "Unable to obtain the X11 root window"
                | "[X11] Screen = {}, root window = {}, geometry = {}x{}, depth = {}"
                | "[X11] Closing X11 display"
        ),
        "generate_textures.rs" => matches!(
            t,
            "Unknown texture family '{}'. Valid families: {}"
                | "Invalid texture buffer: expected {} bytes for {}x{} RGBA8, received {}"
                | "Generated texture is {}x{}; Screenshaver requires {}x{}"
                | "Generated texture contains {} bytes; expected {}"
                | "Texture width cannot be represented as usize"
                | "Texture height cannot be represented as usize"
                | "Texture dimensions overflow the pixel-buffer size"
                | "diagnostic texture generation"
        ),
        "lock_screen_widget.rs" => matches!(
            t,
            "Unable to link lock-screen widget shader: {}"
                | "Lock-screen widget shader contains an interior NUL: {}"
                | "Unable to compile lock-screen widget shader: {}"
                | "Invalid lock-screen widget uniform '{}': {}"
                | "Lock-screen widget shader uniform '{}' was not found"
                | "no shader compiler log was provided"
                | "no program linker log was provided"
        ),
        "render_audio_motion.rs" => matches!(
            t,
            "woofer from hell"
                | "fft mirror warp"
                | "polar propeller"
                | "Unsupported Audio Motion effect '{}'; supported values: off, woofer_from_hell, fft_mirror_warp, polar_propeller"
                | "Invalid uniform name: {}"
                | "Audio Motion shader uniform '{}' was not found"
        ),
        "render_lock_screen_kde.rs" => matches!(
            t,
            "empty CString"
                | "Screenshaver KDE renderer error"
                | "static CString"
                | "Required OpenGL 3.3 entry points were not supplied by the current Qt context"
                | "Qt OpenGL procedure loader was null"
                | "Invalid KDE render size: {width}x{height}"
                | "Unable to load Screenshaver configuration {}: {error}"
                | "Screenshaver KDE renderer handle was null"
        ),
        "render_wallpaper.rs" => matches!(
            t,
            "Uniform name contains an interior null byte: {}"
        ),
        _ => false,
    };

    if batch_v8_invariant {
        return true;
    }

    // Batch v11: logging, localization-engine, duration-parser, post-processing,
    // splash construction, and session-backend diagnostics. These strings report
    // technical implementation state or failures rather than localized presentation.
    // Keep exemptions exact and module-scoped; stable tagged parser telemetry is
    // restricted to its own module.
    let batch_v11_invariant = match filename {
        "logger.rs" => matches!(
            t,
            "[LOGGER] Unable to create directory {} ({})"
                | "[LOGGER] Unable to open {} ({})"
                | "{} {}\\n"
                | "[LOGGER] Unable to write to {} ({})"
                | "[LOGGER] Unable to flush {} ({})"
                | "[L{}] [{}] {}"
                | "[LOGGER] Unable to create log file {} ({})"
                | "[LOGGER] Unable to reset log file {} ({})"
        ),
        "manage_localization.rs" => matches!(
            t,
            "Unable to prepare runtime localization catalog query: {}"
                | "Unable to query runtime localization keys: {}"
                | "Unable to read runtime localization key: {}"
                | "Runtime localization catalog was initialized more than once."
                | "Configured locale '{}' is unavailable and required fallback locale '{}' is not enabled"
                | "Unable to query translation '{}' for locale '{}': {}"
                | "Unable to query canonical English text for localization key '{}': {}"
                | "Unknown localization key '{}'"
                | "Unable to query localization language '{}': {}"
        ),
        "parse_duration.rs" => t.starts_with("[PARSE_DURATION]"),
        "postprocess_shader.rs" => matches!(
            t,
            "Hue rotation {:.3} is outside the supported range {:.1} through {:.1} degrees"
                | "OpenGL could not allocate post-processing benchmark timer queries"
                | "[POSTPROCESS] Requested precision: {}; selected: {} ({}); fallback: {}"
                | "[POSTPROCESS] High-precision render targets were unavailable; standard precision was selected: {}"
                | "[POSTPROCESS] Render scale: {:.3}; scene: {}x{}; output: {}x{}"
                | "Unable to create post-processing targets: high precision failed ({}); standard precision failed ({})"
                | "OpenGL failed to allocate post-processing framebuffer resources"
                | "Post-processing framebuffer is incomplete (OpenGL status 0x{status:04X})"
                | "Render scale {} is outside the supported range {:.2}-{:.2}"
                | "Scaled render dimensions exceed the supported integer range: {:.0}x{:.0}"
                | "Post-processing dimensions must be nonzero, received {}x{}"
                | "Post-processing dimensions exceed OpenGL limits: {}x{}"
        ),
        "render_bloom.rs" => matches!(
            t,
            "Unsupported bloom mode '{}'; supported values: off, audio, spectral, loudness"
                | "Bloom intensity {} is outside the supported range {:.2}-{:.2}"
                | "Bloom saturation {} is outside the supported range {:.2}-{:.2}"
                | "Bloom threshold {} is outside the supported range {:.2}-{:.2}"
                | "Bloom frequency rotation {} is outside the supported range {:.1}-{:.1} degrees"
                | "Unable to build Bloom highlight-extraction program: {}"
                | "Unable to build Bloom audio color-extraction program: {}"
                | "Unable to build Bloom spectral color-extraction program: {}"
                | "Unable to build Bloom blur program: {}"
                | "Unable to build Bloom composition program: {}"
                | "OpenGL failed to allocate the Bloom vertex array"
                | "Bloom post-processing program is missing a required uniform"
        ),
        "splash_screen.rs" => matches!(
            t,
            "Unable to determine splash image format: {}"
                | "Unable to decode splash image: {}"
                | "Splash image has invalid dimensions"
                | "Unable to initialize SDL video for splash screen: {}"
                | "Unable to determine display dimensions: {}"
                | "Unable to create splash window: {}"
                | "Unable to create splash canvas: {}"
                | "Unable to create splash texture: {}"
                | "Unable to upload splash texture: {}"
                | "Unable to draw splash texture: {}"
                | "Unable to create splash event pump: {}"
        ),
        _ => false,
    };

    if batch_v11_invariant {
        return true;
    }

    // GNOME and Wayland names are intentionally scoped to session_backend so an
    // unrelated future module with the same filename does not inherit these exemptions.
    let is_session_backend_module = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        == Some("session_backend");

    if is_session_backend_module
        && filename == "gnome.rs"
        && matches!(
            t,
            "[GNOME] Connecting to session bus"
                | "Failed to connect to GNOME session bus: {}"
                | "[GNOME] Session bus connected"
                | "Failed to create GNOME IdleMonitor proxy: {}"
                | "[GNOME] Mutter IdleMonitor proxy created"
                | "[GNOME] Testing GNOME IdleMonitor availability"
                | "GNOME IdleMonitor unavailable: {}"
                | "[GNOME] GNOME IdleMonitor available"
                | "[GNOME] GNOME backend initialized"
                | "Failed to query GNOME idle time: {}"
        )
    {
        return true;
    }

    if is_session_backend_module
        && filename == "wayland.rs"
        && matches!(
            t,
            "[SESSION] Wayland idle notification: idled"
                | "[SESSION] Wayland idle notification: resumed"
                | "Unable to connect to Wayland compositor: {}"
                | "Unable to read Wayland registry: {}"
                | "Wayland ext_idle_notifier_v1 not advertised"
                | "Wayland wl_seat not advertised"
                | "Unable to complete Wayland object binds: {}"
                | "Wayland idle timeout is too large"
                | "[SESSION] Using Wayland input-only idle notification (protocol v2)"
                | "[SESSION] Using Wayland idle notification (protocol v1)"
                | "Unable to create Wayland idle notification: {}"
                | "Wayland dispatch failed: {}"
        )
    {
        return true;
    }

    // Schema-1 historical-reader validation and SQLite diagnostics are permanent migration compatibility text.
    if filename == "read_schema_v001.rs"
        && matches!(
            t,
            "Unable to count Schema-1 metadata rows: {}"
                | "Schema-1 reader expected exactly one schema_metadata row, found {}"
                | "Unable to read Schema-1 metadata: {}"
                | "Schema-1 reader expected metadata_id 1, found {}"
                | "Schema-1 reader cannot read database schema version {}; expected {}"
                | "Unable to prepare Schema-1 shader read: {}"
                | "Unable to query Schema-1 shaders: {}"
                | "Unable to advance through Schema-1 shaders: {}"
                | "Unable to read Schema-1 shader_id: {}"
                | "Schema-1 reader encountered duplicate shader_id {}"
                | "Unable to read shader_added_at for Schema-1 shader {}: {}"
                | "Unable to read filename for Schema-1 shader {}: {}"
                | "Unable to read source_path for Schema-1 shader {}: {}"
                | "Unable to prepare Schema-1 policy read: {}"
                | "Unable to query Schema-1 policies: {}"
                | "Unable to advance through Schema-1 policies: {}"
                | "Unable to read Schema-1 policy_id: {}"
                | "Schema-1 reader encountered duplicate policy_id {}"
                | "Unable to read shader_id for Schema-1 policy {}: {}"
                | "Schema-1 policy {} references missing shader_id {}"
                | "Unable to read policy_target for Schema-1 policy {}: {}"
                | "Unable to read texture_mode for Schema-1 policy {}: {}"
                | "Unable to read texture_family for Schema-1 policy {}: {}"
                | "Unable to read texture_primitives for Schema-1 policy {}: {}"
                | "Unable to read palette_mode for Schema-1 policy {}: {}"
                | "Unable to read palette_color for Schema-1 policy {}: {}"
                | "Unable to read rendered_fps for Schema-1 policy {}: {}"
                | "Unable to read policy_created_at for Schema-1 policy {}: {}"
                | "Unable to read policy_modified_at for Schema-1 policy {}: {}"
                | "Unable to read policy_name for Schema-1 policy {}: {}"
                | "Unable to read animation_speed for Schema-1 policy {}: {}"
                | "Unable to read starting_offset for Schema-1 policy {}: {}"
                | "Unable to read anti_aliasing for Schema-1 policy {}: {}"
                | "Unable to read dithering for Schema-1 policy {}: {}"
                | "Unable to read color_precision for Schema-1 policy {}: {}"
                | "Unable to read render_scale for Schema-1 policy {}: {}"
                | "Unable to read audiovisual_effect for Schema-1 policy {}: {}"
                | "Unable to read audio_motion_effect for Schema-1 policy {}: {}"
                | "Unable to read bloom_intensity for Schema-1 policy {}: {}"
                | "Unable to read bloom_saturation for Schema-1 policy {}: {}"
                | "Unable to read bloom_threshold for Schema-1 policy {}: {}"
                | "Unable to read bloom_frequency_rotation for Schema-1 policy {}: {}"
                | "Unable to read hue_rotation for Schema-1 policy {}: {}"
                | "Unable to prepare Schema-1 playlist read: {}"
                | "Unable to query Schema-1 playlists: {}"
                | "Unable to advance through Schema-1 playlists: {}"
                | "Unable to read Schema-1 playlist_id: {}"
                | "Schema-1 reader encountered duplicate playlist_id {}"
                | "Unable to read playlist_created_at for Schema-1 playlist {}: {}"
                | "Unable to read playlist_modified_at for Schema-1 playlist {}: {}"
                | "Unable to read playlist_name for Schema-1 playlist {}: {}"
                | "Unable to read description for Schema-1 playlist {}: {}"
                | "Unable to prepare Schema-1 playlist-member read for playlist {}: {}"
                | "Unable to query Schema-1 members for playlist {}: {}"
                | "Unable to advance through Schema-1 members for playlist {}: {}"
                | "Unable to read policy_id from Schema-1 playlist {}: {}"
                | "Unable to read position from Schema-1 playlist {}: {}"
                | "Schema-1 playlist {} references missing policy_id {}"
                | "Unable to count Schema-1 runtime targets: {}"
                | "Schema-1 reader expected exactly two runtime_targets rows, found {}"
                | "Unable to read Schema-1 runtime target '{}': {}"
                | "Schema-1 runtime target '{}' has invalid Single-mode field combination"
                | "Schema-1 runtime target '{}' has invalid Ordered-mode field combination"
                | "Schema-1 runtime target '{}' has invalid Random-mode field combination"
                | "Schema-1 runtime target '{}' has invalid Playlist-mode field combination"
                | "Schema-1 runtime target '{}' contains unsupported display_mode '{}'"
                | "Unable to count Schema-1 app_defaults rows: {}"
                | "Schema-1 reader expected exactly one app_defaults row, found {}"
                | "Unable to read Schema-1 application defaults: {}"
                | "Schema-1 reader expected app_defaults defaults_id 1, found {}"
                | "Unable to count Schema-1 target_defaults rows: {}"
                | "Schema-1 reader expected exactly two target_defaults rows, found {}"
                | "Unable to read Schema-1 target defaults for '{}': {}"
                | "Schema-1 screensaver target defaults require both idle-timeout value and unit"
                | "Schema-1 wallpaper target defaults must not contain an idle timeout"
                | "Schema-1 reader does not recognize target-default target '{}'"
                | "Schema-1 target defaults for '{}' use Random texture mode but contain a texture family"
                | "Schema-1 target defaults for '{}' use Specific texture mode without a texture family"
                | "Schema-1 target defaults for '{}' contain unsupported texture_mode '{}'"
                | "Schema-1 target defaults for '{}' use Random palette mode but contain a palette color"
                | "Schema-1 target defaults for '{}' use Specific palette mode without a palette color"
                | "Schema-1 target defaults for '{}' contain unsupported palette_mode '{}'"
                | "Schema-1 policy {} contains unsupported policy_target '{}'"
                | "Schema-1 policy {} inherits texture settings but contains texture-specific values"
                | "Schema-1 policy {} uses Random texture mode but contains texture-specific values"
                | "Schema-1 policy {} uses Specific texture mode without a texture family"
                | "Schema-1 policy {} uses Specific texture mode without a primitive count"
                | "Schema-1 policy {} contains unsupported texture_mode '{}'"
                | "Schema-1 policy {} inherits palette settings but contains a palette color"
                | "Schema-1 policy {} uses Random palette mode but contains a palette color"
                | "Schema-1 policy {} uses Specific palette mode without a palette color"
                | "Schema-1 policy {} contains unsupported palette_mode '{}'"
                | "Schema-1 runtime target '{}' references missing policy_id {}"
                | "Schema-1 runtime target '{}' references missing playlist_id {}"
                | "Schema-1 runtime target '{}' requires {}"
                | "Schema-1 {} must be positive, found {}"
                | "Schema-1 {} value {} cannot be represented by the migration model"
                | "Schema-1 {} must be 0 or 1, found {}"
                | "Unable to read {} for Schema-1 policy {}: {}"
                | "shader_policies.{}"
        )
    {
        return true;
    }

    // Schema-2 historical-reader validation and SQLite diagnostics are permanent migration compatibility text.
    if filename == "read_schema_v002.rs"
        && matches!(
            t,
            "Unable to count Schema-2 metadata rows: {}"
                | "Schema-2 reader expected exactly one schema_metadata row, found {}"
                | "Unable to read Schema-2 metadata: {}"
                | "Schema-2 reader expected metadata_id 1, found {}"
                | "Schema-2 reader cannot read database schema version {}; expected {}"
                | "Unable to prepare Schema-2 shader read: {}"
                | "Unable to query Schema-2 shaders: {}"
                | "Unable to advance through Schema-2 shaders: {}"
                | "Unable to read Schema-2 shader_id: {}"
                | "Schema-2 reader encountered duplicate shader_id {}"
                | "Unable to read shader_added_at for Schema-2 shader {}: {}"
                | "Unable to read filename for Schema-2 shader {}: {}"
                | "Unable to read source_path for Schema-2 shader {}: {}"
                | "Unable to prepare Schema-2 policy read: {}"
                | "Unable to query Schema-2 policies: {}"
                | "Unable to advance through Schema-2 policies: {}"
                | "Unable to read Schema-2 policy_id: {}"
                | "Schema-2 reader encountered duplicate policy_id {}"
                | "Unable to read shader_id for Schema-2 policy {}: {}"
                | "Schema-2 policy {} references missing shader_id {}"
                | "Unable to read policy_target for Schema-2 policy {}: {}"
                | "Unable to read texture_mode for Schema-2 policy {}: {}"
                | "Unable to read texture_family for Schema-2 policy {}: {}"
                | "Unable to read texture_primitives for Schema-2 policy {}: {}"
                | "Unable to read palette_mode for Schema-2 policy {}: {}"
                | "Unable to read palette_color for Schema-2 policy {}: {}"
                | "Unable to read rendered_fps for Schema-2 policy {}: {}"
                | "Unable to read policy_created_at for Schema-2 policy {}: {}"
                | "Unable to read policy_modified_at for Schema-2 policy {}: {}"
                | "Unable to read policy_name for Schema-2 policy {}: {}"
                | "Unable to read animation_speed for Schema-2 policy {}: {}"
                | "Unable to read starting_offset for Schema-2 policy {}: {}"
                | "Unable to read anti_aliasing for Schema-2 policy {}: {}"
                | "Unable to read dithering for Schema-2 policy {}: {}"
                | "Unable to read color_precision for Schema-2 policy {}: {}"
                | "Unable to read render_scale for Schema-2 policy {}: {}"
                | "Unable to read audiovisual_effect for Schema-2 policy {}: {}"
                | "Unable to read audio_motion_effect for Schema-2 policy {}: {}"
                | "Unable to read bloom_intensity for Schema-2 policy {}: {}"
                | "Unable to read bloom_saturation for Schema-2 policy {}: {}"
                | "Unable to read bloom_threshold for Schema-2 policy {}: {}"
                | "Unable to read bloom_frequency_rotation for Schema-2 policy {}: {}"
                | "Unable to read hue_rotation for Schema-2 policy {}: {}"
                | "Unable to prepare Schema-2 playlist read: {}"
                | "Unable to query Schema-2 playlists: {}"
                | "Unable to advance through Schema-2 playlists: {}"
                | "Unable to read Schema-2 playlist_id: {}"
                | "Schema-2 reader encountered duplicate playlist_id {}"
                | "Unable to read playlist_created_at for Schema-2 playlist {}: {}"
                | "Unable to read playlist_modified_at for Schema-2 playlist {}: {}"
                | "Unable to read playlist_name for Schema-2 playlist {}: {}"
                | "Unable to read description for Schema-2 playlist {}: {}"
                | "Unable to prepare Schema-2 playlist-member read for playlist {}: {}"
                | "Unable to query Schema-2 members for playlist {}: {}"
                | "Unable to advance through Schema-2 members for playlist {}: {}"
                | "Unable to read policy_id from Schema-2 playlist {}: {}"
                | "Unable to read position from Schema-2 playlist {}: {}"
                | "Schema-2 playlist {} references missing policy_id {}"
                | "Unable to count Schema-2 runtime targets: {}"
                | "Schema-2 reader expected exactly two runtime_targets rows, found {}"
                | "Unable to read Schema-2 runtime target '{}': {}"
                | "Schema-2 runtime target '{}' has invalid Single-mode field combination"
                | "Schema-2 runtime target '{}' has invalid Ordered-mode field combination"
                | "Schema-2 runtime target '{}' has invalid Random-mode field combination"
                | "Schema-2 runtime target '{}' has invalid Playlist-mode field combination"
                | "Schema-2 runtime target '{}' contains unsupported display_mode '{}'"
                | "Unable to count Schema-2 app_defaults rows: {}"
                | "Schema-2 reader expected exactly one app_defaults row, found {}"
                | "Unable to read Schema-2 application defaults: {}"
                | "Schema-2 reader expected app_defaults defaults_id 1, found {}"
                | "Unable to count Schema-2 target_defaults rows: {}"
                | "Schema-2 reader expected exactly two target_defaults rows, found {}"
                | "Unable to read Schema-2 target defaults for '{}': {}"
                | "Schema-2 screensaver target defaults require both idle-timeout value and unit"
                | "Schema-2 wallpaper target defaults must not contain an idle timeout"
                | "Schema-2 reader does not recognize target-default target '{}'"
                | "Schema-2 target defaults for '{}' use Random texture mode but contain a texture family"
                | "Schema-2 target defaults for '{}' use Specific texture mode without a texture family"
                | "Schema-2 target defaults for '{}' contain unsupported texture_mode '{}'"
                | "Schema-2 target defaults for '{}' use Random palette mode but contain a palette color"
                | "Schema-2 target defaults for '{}' use Specific palette mode without a palette color"
                | "Schema-2 target defaults for '{}' contain unsupported palette_mode '{}'"
                | "Schema-2 policy {} contains unsupported policy_target '{}'"
                | "Schema-2 policy {} inherits texture settings but contains texture-specific values"
                | "Schema-2 policy {} uses Random texture mode but contains texture-specific values"
                | "Schema-2 policy {} uses Specific texture mode without a texture family"
                | "Schema-2 policy {} uses Specific texture mode without a primitive count"
                | "Schema-2 policy {} contains unsupported texture_mode '{}'"
                | "Schema-2 policy {} inherits palette settings but contains a palette color"
                | "Schema-2 policy {} uses Random palette mode but contains a palette color"
                | "Schema-2 policy {} uses Specific palette mode without a palette color"
                | "Schema-2 policy {} contains unsupported palette_mode '{}'"
                | "Schema-2 runtime target '{}' references missing policy_id {}"
                | "Schema-2 runtime target '{}' references missing playlist_id {}"
                | "Schema-2 runtime target '{}' requires {}"
                | "Schema-2 {} must be positive, found {}"
                | "Schema-2 {} value {} cannot be represented by the migration model"
                | "Schema-2 {} must be 0 or 1, found {}"
                | "Unable to read {} for Schema-2 policy {}: {}"
                | "shader_policies.{}"
        )
    {
        return true;
    }

    // Database-comparison output is developer/diagnostic CLI text whose stable technical vocabulary should remain invariant.
    if filename == "compare_databases.rs"
        && matches!(
            t,
            "BLOB[{}] 0x{}"
                | "Database A"
                | "Database B"
                | "Screenshaver database comparison"
                | "Database A: {}"
                | "Database B: {}"
                | "Comparison mode: semantic (--exclude-metadata --exclude-local-config)"
                | "Comparison mode: semantic (--exclude-metadata)"
                | "Metadata excluded:"
                | "  local shader, policy, playlist, and relationship IDs"
                | "  creation/modification/addition timestamps"
                | "  schema/application provenance metadata"
                | "  derived shader validation/runtime-package metadata"
                | "Semantic relationships are still compared by names, targets, shader paths, and playlist order."
                | "Local configuration excluded:"
                | "  runtime target selections/modes/intervals"
                | "  application defaults"
                | "  screensaver/wallpaper target defaults"
                | "== Semantic table: {} =="
                | "{} semantic row count"
                | "{} row [{}] column {}"
                | "DIFFERENCE {}: {} semantic row [{}] exists only in Database A"
                | "  A: {}"
                | "  B: <missing>"
                | "DIFFERENCE {}: {} semantic row [{}] exists only in Database B"
                | "  A: <missing>"
                | "  B: {}"
                | "Unable to prepare semantic database comparison: {}"
                | "Unable to execute semantic database comparison: {}"
                | "{} occurrence={}"
                | "Comparison mode: literal (--exclude-local-config)"
                | "Comparison mode: literal"
                | "{} does not exist: {}"
                | "{} is not a regular file: {}"
                | "Unable to open {} read-only ({}): {}"
                | "{} integrity check failed to execute: {}"
                | "{} failed SQLite integrity_check: {}"
                | "== Database properties =="
                | "Unable to read {}: {}"
                | "Unable to read SQLite schema: {}"
                | "Unable to query SQLite schema: {}"
                | "Unable to iterate SQLite schema: {}"
                | "== Schema objects =="
                | "{} {} table association"
                | "{} {} definition"
                | "== Table: {} =="
                | "Row comparison skipped because the column layouts differ."
                | "{} row count"
                | "Unable to inspect table '{}': {}"
                | "Unable to read columns for '{}': {}"
                | "Unable to collect columns for '{}': {}"
                | "cid={} type={} not_null={} default={:?} pk={}"
                | "{} column {}"
                | "Unable to prepare row comparison for '{}': {}"
                | "Unable to query rows from '{}': {}"
                | "Unable to collect rows from '{}': {}"
                | "DIFFERENCE {}: {} row [{}] exists only in Database A"
                | "DIFFERENCE {}: {} row [{}] exists only in Database B"
                | "DIFFERENCE {}: {} row occurrence count differs"
                | "  Row: {}"
                | "  A: {} occurrence(s)"
                | "  B: {} occurrence(s)"
                | "DIFFERENCE {}: {}"
        )
    {
        return true;
    }

    // Render-benchmark output is developer/performance diagnostic text, including stable case names, timing labels, and OpenGL diagnostics.
    if filename == "test_render_benchmark.rs"
        && matches!(
            t,
            "Benchmark shader does not exist or is not a file: {}"
                | "Shader '{}' was rejected: {}"
                | "Shader '{}' is unavailable: {}"
                | "Screenshaver Render Benchmark"
                | "Shader: {}"
                | "Processed shader: {}"
                | "Benchmark size: {}x{}"
                | "Warm-up: {} s per case; measurement: {} s per case"
                | "This benchmark intentionally disables the normal FPS limiter and VSync."
                | "Press Esc or close the benchmark window to abort."
                | "SDL initialization failed: {}"
                | "SDL video initialization failed: {}"
                | "Unable to create benchmark window: {}"
                | "Unable to create benchmark OpenGL context: {}"
                | "Benchmark shader compilation failed: {}"
                | "Audio-reactive Bloom is active. Play music during the benchmark to exercise live audio input."
                | "Production baseline"
                | "Postprocess + fence wait"
                | "Postprocess + glFlush"
                | "Postprocess + no explicit sync"
                | "Cached uniforms"
                | "Cached uniforms + no explicit sync"
                | "Raw shader"
                | "Raw shader + no explicit sync"
                | "Unable to create benchmark event pump: {}"
                | "    warming up..."
                | "    measuring..."
                | "Benchmark phase produced no timing result"
                | "    {:.2} FPS, P99 {:.3} ms"
                | "GPU timer-query diagnostic"
                | "This 30-second phase uses OpenGL timestamp queries around the scene and post-processing work."
                | "While it runs, deliberately perform the desktop action that has triggered CRITICAL FPS warnings"
                | "(for example, switch Mango workspaces several times)."
                | "Notes:"
                | "  * 'Production baseline' reproduces the current postprocess + per-frame uniform lookup + glFinish synchronization pattern."
                | "  * Raw-shader cases bypass Screenshaver post-processing and establish the shader's approximate rendering ceiling."
                | "  * PBO is not included in this first harness because the audited production path does not currently perform a per-frame CPU pixel transfer for a PBO to replace."
                | "  * The GPU timer-query diagnostic separates CPU wall time from GPU scene and post-processing time."
                | "  * It also retains the 20 worst CPU-wall and 20 worst GPU-total frames with correlated per-frame measurements."
                | "  * A large CPU-wall spike without a comparable GPU-time spike indicates presentation/compositor/driver waiting rather than an intrinsically slow shader frame."
                | "  * The fence-wait case inserts GL_SYNC_GPU_COMMANDS_COMPLETE after the frame and waits with glClientWaitSync, allowing a direct comparison with the production glFinish barrier."
                | "  * The diagnostic times the actual production post-processing passes individually: primary/AA, Bloom extraction, both blur passes, composition, and dithering/final output."
                | "  * Audio-reactive Bloom requests the normal audio backend and feeds live AudioBands into every postprocessed benchmark phase and the detailed diagnostic."
                | "  * Reduced/fused post-processing and alternate Bloom-resolution experiments can now be designed from measured per-pass costs."
                | "  * Additional outlier tables rank the worst post-processing and scene-shader frames and retain every measured post-processing stage."
                | "  * Diagnostic counters separately identify large CPU/presentation waits, scene-heavy GPU frames, and postprocess-heavy GPU frames."
                | "  * Live AudioBands are printed about once per second and summarized with peak/nonzero counts so Audio Bloom input can be verified objectively."
                | "    [AUDIO] live bands: bass={:.3} midrange={:.3} treble={:.3}"
                | "Audio input diagnostic"
                | "    Samples observed:              {}"
                | "    Nonzero samples:               {}"
                | "    Peak bass:                     {:.3}"
                | "    Peak midrange:                 {:.3}"
                | "    Peak treble:                   {:.3}"
                | "    AUDIO INPUT FAILURE: no nonzero AudioBands were observed while Audio Bloom was requested."
                | "    Audio input confirmed: live nonzero AudioBands reached the benchmark."
                | "Benchmark aborted by user"
                | "Unable to update benchmark animated texture: {}"
                | "OpenGL failed to create the benchmark frame fence"
                | "OpenGL frame-fence wait failed"
                | "Avg FPS"
                | "1% Low"
                | "0.1% Low"
                | "Mean ms"
                | "P99 ms"
                | "Worst ms"
                | "Detailed frame-time percentiles"
                | "Change versus production baseline"
                | "  {:<36} {:+8.2}% average FPS"
                | "  Baseline measured duration: {:.3} s"
                | "Synchronization comparison"
                | "vs baseline"
                | "Fence observation: the scoped fence wait outperformed the production glFinish barrier in this run."
                | "Fence observation: the scoped fence wait did not materially outperform the production glFinish barrier in this run."
                | "OpenGL timer-query objects could not be created"
                | "GPU timer diagnostic produced no samples"
                | "Compos."
                | "GPU total"
                | "GPU timing diagnostic results"
                | "    Frames sampled:                {}"
                | "    CPU wall mean:                 {:.3} ms"
                | "    CPU wall P99:                  {:.3} ms"
                | "    CPU wall worst:                {:.3} ms"
                | "    GPU scene mean:                {:.3} ms"
                | "    GPU postprocess mean:          {:.3} ms"
                | "        Primary / AA:              {:.3} ms"
                | "        Bloom extraction:          {:.3} ms"
                | "        Bloom horizontal blur:     {:.3} ms"
                | "        Bloom vertical blur:       {:.3} ms"
                | "        Bloom composite:           {:.3} ms"
                | "        Dithering / final:         {:.3} ms"
                | "    Postprocess stage shares:"
                | "        Primary / AA:              {:>6.1}%"
                | "        Bloom extraction:          {:>6.1}%"
                | "        Bloom horizontal blur:     {:>6.1}%"
                | "        Bloom vertical blur:       {:>6.1}%"
                | "        Bloom composite:           {:>6.1}%"
                | "        Dithering / final:         {:>6.1}%"
                | "    GPU total mean:                {:.3} ms"
                | "    GPU total P99:                 {:.3} ms"
                | "    GPU total worst:               {:.3} ms"
                | "    Largest CPU-wall minus GPU:    {:.3} ms"
                | "Diagnostic frame classifications"
                | "    External-stall frames (CPU-GPU >= 5 ms): {}"
                | "    Scene-heavy frames (scene >= 5 ms):      {}"
                | "    Postprocess-heavy frames (post >= 2 ms): {}"
                | "    Diagnostic counters only; production Warning/CRITICAL thresholds are unchanged."
                | "Worst CPU-wall frames (correlated measurements)"
                | "Worst GPU-total frames (correlated measurements)"
                | "Worst post-processing frames (individual GPU stages)"
                | "Worst scene-shader frames (individual GPU stages)"
                | "Worst CPU-wall frames (individual GPU stages)"
                | "    Observation: a substantial CPU/presentation wait occurred that was not matched by GPU render time."
                | "    Observation: CPU-wall and GPU timing remained comparatively close during the diagnostic."
                | "CPU wall"
                | "GPU scene"
                | "GPU post"
                | "OpenGL environment:"
                | "    Vendor:   {}"
                | "    Renderer: {}"
                | "    Version:  {}"
        )
    {
        return true;
    }

    // v12 correction: intentionally_invariant() receives trimmed candidate text.
    // These are the exact whitespace-trimmed forms of the remaining reviewed
    // compare_databases.rs developer/diagnostic strings.
    if filename == "compare_databases.rs"
        && matches!(
            t,
            "local shader, policy, playlist, and relationship IDs"
                | "creation/modification/addition timestamps"
                | "schema/application provenance metadata"
                | "derived shader validation/runtime-package metadata"
                | "runtime target selections/modes/intervals"
                | "application defaults"
                | "screensaver/wallpaper target defaults"
                | "A: {}"
                | "B: <missing>"
                | "A: <missing>"
                | "B: {}"
                | "Row: {}"
                | "A: {} occurrence(s)"
                | "B: {} occurrence(s)"
        )
    {
        return true;
    }

    // v12 correction: intentionally_invariant() receives trimmed candidate text.
    // These are the exact whitespace-trimmed forms of the remaining reviewed
    // test_render_benchmark.rs developer/diagnostic strings.
    if filename == "test_render_benchmark.rs"
        && matches!(
            t,
            "warming up..."
                | "measuring..."
                | "{:.2} FPS, P99 {:.3} ms"
                | "* 'Production baseline' reproduces the current postprocess + per-frame uniform lookup + glFinish synchronization pattern."
                | "* Raw-shader cases bypass Screenshaver post-processing and establish the shader's approximate rendering ceiling."
                | "* PBO is not included in this first harness because the audited production path does not currently perform a per-frame CPU pixel transfer for a PBO to replace."
                | "* The GPU timer-query diagnostic separates CPU wall time from GPU scene and post-processing time."
                | "* It also retains the 20 worst CPU-wall and 20 worst GPU-total frames with correlated per-frame measurements."
                | "* A large CPU-wall spike without a comparable GPU-time spike indicates presentation/compositor/driver waiting rather than an intrinsically slow shader frame."
                | "* The fence-wait case inserts GL_SYNC_GPU_COMMANDS_COMPLETE after the frame and waits with glClientWaitSync, allowing a direct comparison with the production glFinish barrier."
                | "* The diagnostic times the actual production post-processing passes individually: primary/AA, Bloom extraction, both blur passes, composition, and dithering/final output."
                | "* Audio-reactive Bloom requests the normal audio backend and feeds live AudioBands into every postprocessed benchmark phase and the detailed diagnostic."
                | "* Reduced/fused post-processing and alternate Bloom-resolution experiments can now be designed from measured per-pass costs."
                | "* Additional outlier tables rank the worst post-processing and scene-shader frames and retain every measured post-processing stage."
                | "* Diagnostic counters separately identify large CPU/presentation waits, scene-heavy GPU frames, and postprocess-heavy GPU frames."
                | "* Live AudioBands are printed about once per second and summarized with peak/nonzero counts so Audio Bloom input can be verified objectively."
                | "[AUDIO] live bands: bass={:.3} midrange={:.3} treble={:.3}"
                | "Samples observed:              {}"
                | "Nonzero samples:               {}"
                | "Peak bass:                     {:.3}"
                | "Peak midrange:                 {:.3}"
                | "Peak treble:                   {:.3}"
                | "AUDIO INPUT FAILURE: no nonzero AudioBands were observed while Audio Bloom was requested."
                | "Audio input confirmed: live nonzero AudioBands reached the benchmark."
                | "{:<36} {:+8.2}% average FPS"
                | "Baseline measured duration: {:.3} s"
                | "Frames sampled:                {}"
                | "CPU wall mean:                 {:.3} ms"
                | "CPU wall P99:                  {:.3} ms"
                | "CPU wall worst:                {:.3} ms"
                | "GPU scene mean:                {:.3} ms"
                | "GPU postprocess mean:          {:.3} ms"
                | "Primary / AA:              {:.3} ms"
                | "Bloom extraction:          {:.3} ms"
                | "Bloom horizontal blur:     {:.3} ms"
                | "Bloom vertical blur:       {:.3} ms"
                | "Bloom composite:           {:.3} ms"
                | "Dithering / final:         {:.3} ms"
                | "Postprocess stage shares:"
                | "Primary / AA:              {:>6.1}%"
                | "Bloom extraction:          {:>6.1}%"
                | "Bloom horizontal blur:     {:>6.1}%"
                | "Bloom vertical blur:       {:>6.1}%"
                | "Bloom composite:           {:>6.1}%"
                | "Dithering / final:         {:>6.1}%"
                | "GPU total mean:                {:.3} ms"
                | "GPU total P99:                 {:.3} ms"
                | "GPU total worst:               {:.3} ms"
                | "Largest CPU-wall minus GPU:    {:.3} ms"
                | "External-stall frames (CPU-GPU >= 5 ms): {}"
                | "Scene-heavy frames (scene >= 5 ms):      {}"
                | "Postprocess-heavy frames (post >= 2 ms): {}"
                | "Diagnostic counters only; production Warning/CRITICAL thresholds are unchanged."
                | "Observation: a substantial CPU/presentation wait occurred that was not matched by GPU render time."
                | "Observation: CPU-wall and GPU timing remained comparatively close during the diagnostic."
                | "Vendor:   {}"
                | "Renderer: {}"
                | "Version:  {}"
        )
    {
        return true;
    }


    // Batch v13: generator, native-lock, GLX, renderer, session-backend, and ISF
    // preprocessing strings below are technical diagnostics, structured telemetry,
    // test labels, machine identifiers, or shader-source transformation text. Keep
    // exemptions module-scoped so future presentation prose remains auditable.
    if filename == "generate_skulls.rs"
        && matches!(
            t,
            "Skull source image has zero width or height"
                | "Skull texture buffer size overflow"
                | "Unable to decode embedded skull PNG: {}"
                | "Embedded skull PNG contains no visible pixels"
                | "embedded skull PNG"
                | "minimum skull layout"
                | "explicit minimum skull layout"
                | "maximum skull layout"
                | "explicit maximum skull layout"
                | "sparse skull layout"
                | "dense skull layout"
                | "first skull texture"
                | "second skull texture"
                | "skull generation"
        )
    {
        return true;
    }

    if filename == "glx_context.rs"
        && matches!(
            t,
            "Cannot choose a GLX framebuffer configuration for a null X11 display."
                | "Choosing GLX framebuffer configuration..."
                | "glXChooseFBConfig() did not return a compatible framebuffer configuration."
                | "glXGetVisualFromFBConfig() failed for the selected framebuffer configuration."
                | "Selected compatible GLX framebuffer configuration and X11 visual."
                | "Cannot create a GLX context for a null X11 display."
                | "Creating GLX context..."
                | "glXCreateNewContext() failed."
                | "Created GLX context."
                | "Cannot activate a GLX context on a null X11 display."
                | "Making GLX context current..."
                | "glXMakeContextCurrent() failed."
                | "GLX context is current."
                | "Cannot release a GLX context from a null X11 display."
                | "glXMakeContextCurrent() failed while releasing the current context."
                | "Destroying GLX context..."
                | "Destroyed GLX context."
        )
    {
        return true;
    }

    if filename == "manage_screen_lock_gnome.rs"
        && (t.starts_with("[LOCK] ")
            || matches!(
                t,
                "Unable to connect to the GNOME session bus: {}"
                    | "Unable to create GNOME ScreenSaver D-Bus proxy: {}"
                    | "Wallpaper renderer did not acknowledge pause before GNOME screen lock"
                    | "GNOME Shell screen-lock request failed: {}"
                    | "GNOME Shell did not confirm an active screen lock within {} seconds"
                    | "Unable to query GNOME screen-lock state: {}"
            ))
    {
        return true;
    }

    if filename == "manage_screen_lock_xfce.rs"
        && (t.starts_with("[LOCK] ")
            || matches!(
                t,
                "Unable to connect to the XFCE session bus: {}"
                    | "Unable to create XFCE ScreenSaver D-Bus proxy: {}"
                    | "Wallpaper renderer did not acknowledge pause before XFCE screen lock"
                    | "XFCE screen-lock request failed: {}"
                    | "XFCE did not confirm an active screen lock within {} seconds"
                    | "Unable to query XFCE screen-lock state: {}"
            ))
    {
        return true;
    }

    if filename == "notify_wallpaper.rs"
        && (t.starts_with("[WALLPAPER] ")
            || matches!(
                t,
                "could not connect to the session D-Bus: {}"
                    | "D-Bus notification failed: {}"
                    | "could not read notification ID: {}"
                    | "D-Bus CloseNotification failed: {}"
            ))
    {
        return true;
    }

    if filename == "preprocess_isf.rs"
        && matches!(
            t,
            "ISF multipass shaders are not supported in this implementation"
                | "ISF imported resources are not supported in this implementation"
                | "uniform {} {};"
                | "{}\\n{}"
                | "ISF input has an invalid GLSL identifier: {}"
                | "ISF image input '{}' is not supported yet"
                | "ISF audio input '{}' is not supported yet"
                | "ISF event input '{}' is not supported yet"
                | "Unsupported ISF input type '{}' for '{}'"
                | "(?ms)^\\s*#ifdef\\s+GL_ES\\s*\\n\\s*precision\\s+\\w+\\s+float\\s*;\\s*\\n\\s*#endif\\s*\\n?"
                | "ISF GL_ES precision block regex"
                | "(?m)^[ \\t]*(?:const[ \\t]+)?vec3[ \\t]+iResolution[ \\t]*=[ \\t]*vec3[ \\t]*\\([ \\t]*RENDERSIZE[ \\t]*,[ \\t]*1(?:\\.0*)?[ \\t]*\\)[ \\t]*;[ \\t]*(?://[^\\n]*)?\\n?"
                | "ISF redundant iResolution alias regex"
                | "(?m)^[ \\t]*(?:const[ \\t]+)?float[ \\t]+iTime[ \\t]*=[ \\t]*TIME[ \\t]*;[ \\t]*(?://[^\\n]*)?\\n?"
                | "ISF redundant iTime alias regex"
                | "ISF main function regex"
                | "fragColor = vec4(0.0)"
                | "\\n    fragColor = vec4(0.0);"
        )
    {
        return true;
    }

    if filename == "render_frame.rs"
        && (t.starts_with("[RENDER] ")
            || matches!(
                t,
                "Failed to create SDL event pump: {error}"
                    | "Failed to create renderer window: {error}"
                    | "Failed to create OpenGL context: {error}"
            ))
    {
        return true;
    }

    // session_backend/mod.rs contains only backend-selection telemetry and the
    // final technical failure returned when no supported idle backend can start.
    if path.ends_with("session_backend/mod.rs")
        && (t.starts_with("[SESSION] ")
            || t.starts_with("[LOCK] ")
            || t == "No compatible session backend available")
    {
        return true;
    }

    // Previously reviewed one-line residuals. These are a logger timestamp token,
    // an SQL fragment, an internal import conflict-state token, and the standalone
    // KDE renderer host's crate-local database diagnostic.
    if filename == "logger.rs"
        && t == "UNIX-{}"
    {
        return true;
    }

    if filename == "query_database.rs"
        && t == "\\n             WHERE"
    {
        return true;
    }

    if filename == "import_data.rs"
        && t == "Receiving installation"
    {
        return true;
    }

    if filename == "lib.rs"
        && t == "KDE renderer host requires an existing Screenshaver database at '{}'"
    {
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
