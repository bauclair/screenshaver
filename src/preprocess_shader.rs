use regex::Regex;

const MAX_SHADER_SOURCE_BYTES: usize = 1_000_000;
const MAX_CONSTANT_LOOP_BOUND: u64 = 2048;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
)]
pub struct ShaderChannelUsage {
    pub channels: [bool; 4],
    pub requires_mipmaps: bool,
}

impl ShaderChannelUsage {
    pub fn uses_any_channel(
        self,
    ) -> bool {
        self.channels
            .iter()
            .any(
                |used| {
                    *used
                }
            )
    }
}

#[derive(Debug, Default)]
pub struct PreprocessResult {
    pub source: String,
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
    pub rejection_reasons: Vec<String>,
    pub channel_usage: ShaderChannelUsage,
}

pub fn preprocess_shader(source: &str) -> String {
    preprocess_shader_with_report(source).source
}

pub fn preprocess_shader_with_report(source: &str) -> PreprocessResult {
    let mut applied = Vec::new();
    let mut warnings = Vec::new();
    let mut rejection_reasons = Vec::new();

    let mut processed = normalize_line_endings(source);
    processed = cleanup_version_and_precision_lines(&processed);
    processed = glsl_comp_0001_replace_texture2d(&processed, &mut applied);
    processed = glsl_comp_0002_replace_gl_frag_color(&processed, &mut applied);
    processed = glsl_comp_0003_strip_float_suffixes(&processed, &mut applied);
    processed = glsl_comp_0006_initialize_multi_declarations(&processed, &mut applied);
    processed = glsl_comp_0004_initialize_for_loop_counters(&processed, &mut applied);
    processed = glsl_comp_0005_initialize_main_image_output(&processed, &mut applied);
    processed = initialize_simple_accumulators(&processed, &mut applied);
    processed = initialize_partial_vectors(&processed, &mut applied);
    processed = glsl_comp_0007_repair_malformed_vec3(&processed, &mut applied);

    analyze_warnings(&processed, &mut warnings);
    analyze_rejection_risks(&processed, true, &mut rejection_reasons);

    let channel_usage =
        analyze_channel_usage(
            &processed
        );

    PreprocessResult {
        source: wrap_shadertoy_main_image(&processed),
        applied,
        warnings,
        rejection_reasons,
        channel_usage,
    }
}

pub fn analyze_native_shader(source: &str) -> (Vec<String>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut rejection_reasons = Vec::new();
    analyze_warnings(source, &mut warnings);
    analyze_rejection_risks(source, false, &mut rejection_reasons);
    (warnings, rejection_reasons)
}

pub fn analyze_native_channel_usage(
    source: &str,
) -> ShaderChannelUsage {
    analyze_channel_usage(
        source
    )
}

fn normalize_line_endings(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn cleanup_version_and_precision_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("#version") && !trimmed.starts_with("precision ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ------------------------------------------------------------
// GLSL-COMP-0001
// Title: Replace legacy texture2D() calls.
// Category: Compatibility Rewrite
// Status: Stable
// Introduced: Screenshaver pre-release
// Implemented: Before 2026-07-11
// Last Revised: 2026-07-11
// ------------------------------------------------------------
fn glsl_comp_0001_replace_texture2d(source: &str, applied: &mut Vec<String>) -> String {
    replace_code_identifier(source, "texture2D", "texture", applied)
}

// ------------------------------------------------------------
// GLSL-COMP-0002
// Title: Replace legacy gl_FragColor output references.
// Category: Compatibility Rewrite
// Status: Stable
// Introduced: Screenshaver pre-release
// Implemented: Before 2026-07-11
// Last Revised: 2026-07-11
// ------------------------------------------------------------
fn glsl_comp_0002_replace_gl_frag_color(source: &str, applied: &mut Vec<String>) -> String {
    replace_code_identifier(source, "gl_FragColor", "fragColor", applied)
}

// ------------------------------------------------------------
// GLSL-COMP-0003
// Title: Remove unsupported C-style float suffixes.
// Category: Compatibility Rewrite
// Status: Stable
// Introduced: Screenshaver pre-release
// Implemented: Before 2026-07-11
// Last Revised: 2026-07-11
// ------------------------------------------------------------
fn glsl_comp_0003_strip_float_suffixes(source: &str, applied: &mut Vec<String>) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(
        r"(?i)(?:\d+\.\d*|\.\d+|\d+[eE][+-]?\d+|\d+\.\d*[eE][+-]?\d+)[fF]\b",
    )
    .expect("float-suffix compatibility regex");

    let edits = regex
        .find_iter(&mask)
        .map(|matched| Edit {
            start: matched.end() - 1,
            end: matched.end(),
            replacement: String::new(),
        })
        .collect::<Vec<_>>();

    if !edits.is_empty() {
        applied.push("GLSL-COMP-0003:strip-float-suffixes".into());
    }

    apply_edits(source, edits)
}

// ------------------------------------------------------------
// GLSL-COMP-0004
// Title: Initialize uninitialized for-loop counters.
// Category: Compatibility Rewrite
// Status: Stable
// Introduced: Screenshaver pre-release
// Implemented: Before 2026-07-11
// Last Revised: 2026-07-11
// ------------------------------------------------------------
fn glsl_comp_0004_initialize_for_loop_counters(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(r"for\s*\(\s*(int|float)\s+([A-Za-z_]\w*)\s*;")
        .expect("for-loop counter compatibility regex");

    let mut edits = Vec::new();

    for captures in regex.captures_iter(&mask) {
        let whole = captures.get(0).expect("complete for-loop counter match");
        let variable_type = captures.get(1).expect("for-loop counter type").as_str();
        let variable_name = captures.get(2).expect("for-loop counter name").as_str();
        let zero = if variable_type == "int" { "0" } else { "0.0" };

        edits.push(Edit {
            start: whole.start(),
            end: whole.end(),
            replacement: format!("for ({variable_type} {variable_name} = {zero};"),
        });

        applied.push(format!(
            "GLSL-COMP-0004:initialize-loop-counter:{variable_name}"
        ));
    }

    apply_edits(source, edits)
}

// ------------------------------------------------------------
// GLSL-COMP-0005
// Title: Initialize the mainImage output parameter.
// Category: Compatibility Rewrite
// Status: Stable
// Introduced: Screenshaver pre-release
// Implemented: Before 2026-07-11
// Last Revised: 2026-07-11
// ------------------------------------------------------------
fn glsl_comp_0005_initialize_main_image_output(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(
        r"void\s+mainImage\s*\(\s*out\s+vec4\s+([A-Za-z_]\w*)\s*,[^)]*\)\s*\{",
    )
    .expect("mainImage output compatibility regex");

    let Some(captures) = regex.captures(&mask) else {
        return source.to_string();
    };

    let whole = captures.get(0).expect("complete mainImage signature match");
    let output_name = captures.get(1).expect("mainImage output variable").as_str();
    let nearby_end = (whole.end() + 192).min(mask.len());
    let nearby = &mask[whole.end()..nearby_end];

    let already_initialized = Regex::new(&format!(
        r"\b{}\s*=\s*vec4\s*\(\s*0(?:\.0)?\s*\)",
        regex::escape(output_name)
    ))
    .expect("existing mainImage initialization regex")
    .is_match(nearby);

    if already_initialized {
        return source.to_string();
    }

    applied.push(format!(
        "GLSL-COMP-0005:initialize-mainImage-output:{output_name}"
    ));

    apply_edits(
        source,
        vec![Edit {
            start: whole.end(),
            end: whole.end(),
            replacement: format!("\n    {output_name} = vec4(0.0);"),
        }],
    )
}

// ------------------------------------------------------------
// GLSL-COMP-0006
// Title: Initialize unassigned multi-variable declarations.
// Category: Compatibility Rewrite
// Status: Experimental
// Introduced: Screenshaver pre-release
// First Observed: 2026-07-11
// Implemented: 2026-07-11
// Last Revised: 2026-07-11
// First Cases: shell/cube-carving shader; Dark Transit
// ------------------------------------------------------------
fn glsl_comp_0006_initialize_multi_declarations(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let mask = code_mask(source);
    let declaration_regex = Regex::new(
        r"(?m)^([ \t]*)(float|int|vec2|vec3|vec4)\s+([^;\n]+);",
    )
    .expect("GLSL-COMP-0006 declaration regex");
    let identifier_regex = Regex::new(r"^[A-Za-z_]\w*$")
        .expect("GLSL-COMP-0006 identifier regex");

    let mut edits = Vec::new();

    for captures in declaration_regex.captures_iter(&mask) {
        let whole = captures.get(0).expect("GLSL-COMP-0006 complete declaration");
        let indentation = captures.get(1).expect("GLSL-COMP-0006 indentation").as_str();
        let variable_type = captures.get(2).expect("GLSL-COMP-0006 variable type").as_str();
        let declarator_match = captures.get(3).expect("GLSL-COMP-0006 declarator list");
        let masked_declarators = declarator_match.as_str();
        let declarator_ranges = split_top_level_declarators(masked_declarators);

        if declarator_ranges.len() < 2 {
            continue;
        }

        let original_declarators =
            &source[declarator_match.start()..declarator_match.end()];

        let mut rewritten_parts = Vec::new();
        let mut initialized_names = Vec::new();
        let mut declaration_is_safe = true;

        for (start, end) in declarator_ranges {
            let masked_part = masked_declarators[start..end].trim();
            let original_part = original_declarators[start..end].trim();

            if has_top_level_assignment(masked_part) {
                rewritten_parts.push(original_part.to_string());
                continue;
            }

            if !identifier_regex.is_match(masked_part) {
                declaration_is_safe = false;
                break;
            }

            rewritten_parts.push(format!(
                "{} = {}",
                original_part,
                zero_value(variable_type)
            ));
            initialized_names.push(masked_part.to_string());
        }

        if !declaration_is_safe || initialized_names.is_empty() {
            continue;
        }

        edits.push(Edit {
            start: whole.start(),
            end: whole.end(),
            replacement: format!(
                "{}{} {};",
                indentation,
                variable_type,
                rewritten_parts.join(", ")
            ),
        });

        for variable_name in initialized_names {
            applied.push(format!(
                "GLSL-COMP-0006:initialize-multi-declarator:{variable_name}"
            ));
        }
    }

    apply_edits(source, edits)
}

// ------------------------------------------------------------
// GLSL-COMP-0007
// Title: Repair malformed vec3(V.z, 0, -V) constructors.
// Category: Compatibility Rewrite
// Status: Experimental
// Introduced: Screenshaver pre-release
// First Observed: 2026-07-11
// Implemented: 2026-07-11
// Last Revised: 2026-07-11
// First Case: Dark Transit
// ------------------------------------------------------------
fn glsl_comp_0007_repair_malformed_vec3(
    source: &str,
    applied: &mut Vec<String>,
) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(
        r"vec3\s*\(\s*([A-Za-z_]\w*)\.z\s*,\s*0(?:\.0)?\s*,\s*-\s*([A-Za-z_]\w*)\s*\)",
    )
    .expect("GLSL-COMP-0007 malformed vec3 regex");

    let mut edits = Vec::new();

    for captures in regex.captures_iter(&mask) {
        let whole = captures.get(0).expect("GLSL-COMP-0007 complete constructor");
        let first_identifier = captures.get(1).expect("GLSL-COMP-0007 first identifier").as_str();
        let second_identifier = captures.get(2).expect("GLSL-COMP-0007 second identifier").as_str();

        if first_identifier != second_identifier {
            continue;
        }

        edits.push(Edit {
            start: whole.start(),
            end: whole.end(),
            replacement: format!(
                "vec3({}.z, 0.0, -{}.x)",
                first_identifier,
                first_identifier
            ),
        });

        applied.push(format!(
            "GLSL-COMP-0007:repair-malformed-vec3:{first_identifier}"
        ));
    }

    apply_edits(source, edits)
}

fn initialize_simple_accumulators(source: &str, applied: &mut Vec<String>) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(
        r"(?m)^([ \t]*)(float|int|vec2|vec3|vec4)\s+([A-Za-z_]\w*)\s*;",
    )
    .expect("simple accumulator declaration regex");

    let mut edits = Vec::new();

    for captures in regex.captures_iter(&mask) {
        let whole = captures.get(0).expect("complete accumulator declaration");
        let indentation = captures.get(1).expect("accumulator indentation").as_str();
        let variable_type = captures.get(2).expect("accumulator type").as_str();
        let variable_name = captures.get(3).expect("accumulator name").as_str();
        let scope_end = end_of_current_scope(&mask, whole.end());
        let tail = &mask[whole.end()..scope_end];
        let compound_use = first_compound_use(tail, variable_name);
        let plain_assignment = first_plain_assignment(tail, variable_name);
        let read_before_write = match (compound_use, plain_assignment) {
            (Some(use_position), Some(assignment_position)) => use_position < assignment_position,
            (Some(_), None) => true,
            _ => false,
        };

        if read_before_write {
            edits.push(Edit {
                start: whole.start(),
                end: whole.end(),
                replacement: format!(
                    "{indentation}{variable_type} {variable_name} = {};",
                    zero_value(variable_type)
                ),
            });
            applied.push(format!("initialize-accumulator:{variable_name}"));
        }
    }

    apply_edits(source, edits)
}

fn initialize_partial_vectors(source: &str, applied: &mut Vec<String>) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(
        r"(?m)^([ \t]*)(vec2|vec3|vec4)\s+([A-Za-z_]\w*)\s*;",
    )
    .expect("partial-vector declaration regex");

    let mut edits = Vec::new();

    for captures in regex.captures_iter(&mask) {
        let whole = captures.get(0).expect("complete partial-vector declaration");
        let indentation = captures.get(1).expect("partial-vector indentation").as_str();
        let variable_type = captures.get(2).expect("partial-vector type").as_str();
        let variable_name = captures.get(3).expect("partial-vector name").as_str();
        let scope_end = end_of_current_scope(&mask, whole.end());
        let tail = &mask[whole.end()..scope_end];

        let component_assignment = Regex::new(&format!(
            r"\b{}\s*\.\s*[xyzwrgba]{{1,4}}\s*=",
            regex::escape(variable_name)
        ))
        .expect("partial-vector component assignment regex")
        .find(tail)
        .map(|matched| matched.start());

        let full_assignment = first_plain_assignment(tail, variable_name);
        let partial_before_full = match (component_assignment, full_assignment) {
            (Some(partial_position), Some(full_position)) => partial_position < full_position,
            (Some(_), None) => true,
            _ => false,
        };

        if partial_before_full {
            edits.push(Edit {
                start: whole.start(),
                end: whole.end(),
                replacement: format!(
                    "{indentation}{variable_type} {variable_name} = {variable_type}(0.0);"
                ),
            });
            applied.push(format!("initialize-partial-vector:{variable_name}"));
        }
    }

    apply_edits(source, edits)
}

fn analyze_channel_usage(
    source: &str,
) -> ShaderChannelUsage {
    let mask =
        code_mask(
            source
        );

    let mut channels =
        [false; 4];

    for (
        index,
        used,
    ) in channels
        .iter_mut()
        .enumerate()
    {
        let channel_name =
            format!(
                "iChannel{index}"
            );

        *used =
            contains_identifier(
                &mask,
                &channel_name,
            );
    }

    ShaderChannelUsage {
        channels,
        requires_mipmaps:
            contains_identifier(
                &mask,
                "textureLod",
            ),
    }
}

fn analyze_warnings(source: &str, warnings: &mut Vec<String>) {
    let mask = code_mask(source);

    if contains_identifier(&mask, "iTimeDelta") {
        warnings.push("uses-iTimeDelta: renderer upload still required".into());
    }
    if contains_identifier(&mask, "iChannelResolution") {
        warnings.push("uses-iChannelResolution: renderer upload still required".into());
    }
    if contains_identifier(&mask, "textureLod") {
        warnings.push("uses-textureLod: channel requires mipmaps".into());
    }

    for index in 0..4 {
        let channel_name = format!("iChannel{index}");
        if contains_identifier(&mask, &channel_name) {
            warnings.push(format!(
                "uses-{channel_name}: valid bound texture required"
            ));
        }
    }

    if Regex::new(r"<\s*(100|128|256|500|1000)\b")
        .expect("high-cost-loop warning regex")
        .is_match(&mask)
    {
        warnings.push("high-cost-loop".into());
    }

    let suspicious_constructor = Regex::new(
        r"vec3\s*\(\s*([A-Za-z_]\w*)\.z\s*,\s*0(?:\.0)?\s*,\s*-\s*([A-Za-z_]\w*)\s*\)",
    )
    .expect("malformed vec3 warning regex");

    for captures in suspicious_constructor.captures_iter(&mask) {
        let first_identifier = captures.get(1).expect("first malformed vec3 identifier").as_str();
        let second_identifier = captures.get(2).expect("second malformed vec3 identifier").as_str();
        if first_identifier == second_identifier {
            warnings.push(format!(
                "malformed-vector-constructor: inspect {}",
                captures.get(0).expect("complete malformed vec3 match").as_str()
            ));
        }
    }

    warnings.sort();
    warnings.dedup();
}

fn analyze_rejection_risks(
    source: &str,
    require_main_image: bool,
    reasons: &mut Vec<String>,
) {
    let mask = code_mask(source);

    if source.len() > MAX_SHADER_SOURCE_BYTES {
        reasons.push(format!(
            "Shader source is too large: {} bytes exceeds {}",
            source.len(),
            MAX_SHADER_SOURCE_BYTES
        ));
    }

    if Regex::new(r"\bwhile\s*\(\s*(true|1)\s*\)")
        .expect("nonterminating while-loop regex")
        .is_match(&mask)
    {
        reasons.push("Potentially nonterminating while loop detected".into());
    }

    if Regex::new(r"\bfor\s*\(\s*;\s*;\s*\)")
        .expect("nonterminating for-loop regex")
        .is_match(&mask)
    {
        reasons.push("Potentially nonterminating for (;;) loop detected".into());
    }

    let constant_loop_bound = Regex::new(r"\bfor\s*\([^;]*;[^;]*<\s*(\d+)\s*;")
        .expect("constant loop-bound regex");

    for captures in constant_loop_bound.captures_iter(&mask) {
        let bound_text = captures.get(1).expect("constant loop bound").as_str();
        if let Ok(bound) = bound_text.parse::<u64>() {
            if bound > MAX_CONSTANT_LOOP_BOUND {
                reasons.push(format!(
                    "Constant loop bound {bound} exceeds safety limit {MAX_CONSTANT_LOOP_BOUND}"
                ));
            }
        }
    }

    if require_main_image
        && !Regex::new(r"\bvoid\s+mainImage\s*\(")
            .expect("mainImage requirement regex")
            .is_match(&mask)
    {
        reasons.push("ShaderToy source does not define mainImage()".into());
    }

    if let Some(reason) = structural_error(&mask) {
        reasons.push(reason);
    }

    reasons.sort();
    reasons.dedup();
}

fn structural_error(source: &str) -> Option<String> {
    let mut braces = 0_i32;
    let mut parentheses = 0_i32;
    let mut brackets = 0_i32;

    for byte in source.bytes() {
        match byte {
            b'{' => braces += 1,
            b'}' => {
                braces -= 1;
                if braces < 0 {
                    return Some("Unbalanced closing brace detected".into());
                }
            }
            b'(' => parentheses += 1,
            b')' => {
                parentheses -= 1;
                if parentheses < 0 {
                    return Some("Unbalanced closing parenthesis detected".into());
                }
            }
            b'[' => brackets += 1,
            b']' => {
                brackets -= 1;
                if brackets < 0 {
                    return Some("Unbalanced closing bracket detected".into());
                }
            }
            _ => {}
        }
    }

    if braces != 0 {
        Some("Unbalanced braces detected".into())
    } else if parentheses != 0 {
        Some("Unbalanced parentheses detected".into())
    } else if brackets != 0 {
        Some("Unbalanced brackets detected".into())
    } else {
        None
    }
}

fn wrap_shadertoy_main_image(source: &str) -> String {
    format!(
        r#"#version 330 core

out vec4 fragColor;

uniform float iTime;
uniform float iTimeDelta;
uniform vec3 iResolution;
uniform vec3 iChannelResolution[4];
uniform vec4 iMouse;
uniform int iFrame;
uniform sampler2D iChannel0;
uniform sampler2D iChannel1;
uniform sampler2D iChannel2;
uniform sampler2D iChannel3;

{}

void main() {{
    mainImage(fragColor, gl_FragCoord.xy);
    fragColor.a = 1.0;
}}
"#,
        source
    )
}

#[derive(Debug)]
struct Edit {
    start: usize,
    end: usize,
    replacement: String,
}

fn apply_edits(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by(|left, right| right.start.cmp(&left.start));
    let mut output = source.to_string();
    for edit in edits {
        output.replace_range(edit.start..edit.end, &edit.replacement);
    }
    output
}

fn replace_code_identifier(
    source: &str,
    original: &str,
    replacement: &str,
    applied: &mut Vec<String>,
) -> String {
    let mask = code_mask(source);
    let regex = Regex::new(&format!(r"\b{}\b", regex::escape(original)))
        .expect("code-identifier replacement regex");

    let edits = regex
        .find_iter(&mask)
        .map(|matched| Edit {
            start: matched.start(),
            end: matched.end(),
            replacement: replacement.to_string(),
        })
        .collect::<Vec<_>>();

    if !edits.is_empty() {
        applied.push(format!("replace-{original}"));
    }

    apply_edits(source, edits)
}

fn contains_identifier(source: &str, identifier: &str) -> bool {
    Regex::new(&format!(r"\b{}\b", regex::escape(identifier)))
        .expect("identifier-detection regex")
        .is_match(source)
}

fn zero_value(variable_type: &str) -> String {
    match variable_type {
        "float" => "0.0".into(),
        "int" => "0".into(),
        _ => format!("{variable_type}(0.0)"),
    }
}

fn split_top_level_declarators(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut ranges = Vec::new();
    let mut range_start = 0;
    let mut parentheses = 0_i32;
    let mut brackets = 0_i32;
    let mut braces = 0_i32;

    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' => parentheses += 1,
            b')' => parentheses -= 1,
            b'[' => brackets += 1,
            b']' => brackets -= 1,
            b'{' => braces += 1,
            b'}' => braces -= 1,
            b',' if parentheses == 0 && brackets == 0 && braces == 0 => {
                ranges.push((range_start, index));
                range_start = index + 1;
            }
            _ => {}
        }
    }

    ranges.push((range_start, source.len()));
    ranges
}

fn has_top_level_assignment(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut parentheses = 0_i32;
    let mut brackets = 0_i32;
    let mut braces = 0_i32;

    for byte in bytes {
        match byte {
            b'(' => parentheses += 1,
            b')' => parentheses -= 1,
            b'[' => brackets += 1,
            b']' => brackets -= 1,
            b'{' => braces += 1,
            b'}' => braces -= 1,
            b'=' if parentheses == 0 && brackets == 0 && braces == 0 => {
                return true;
            }
            _ => {}
        }
    }

    false
}

fn end_of_current_scope(mask: &str, start: usize) -> usize {
    let bytes = mask.as_bytes();
    let mut depth = 0_i32;

    for byte in &bytes[..start] {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
    }

    let target_depth = depth;

    for (offset, byte) in bytes[start..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth < target_depth {
                    return start + offset;
                }
            }
            _ => {}
        }
    }

    mask.len()
}

fn first_compound_use(source: &str, variable_name: &str) -> Option<usize> {
    let escaped_name = regex::escape(variable_name);
    let suffix = Regex::new(&format!(
        r"\b{escaped_name}\s*(\+=|-=|\*=|/=|%=|\+\+|--)"
    ))
    .expect("compound suffix-use regex")
    .find(source)
    .map(|matched| matched.start());

    let prefix = Regex::new(&format!(r"(\+\+|--)\s*\b{escaped_name}\b"))
        .expect("compound prefix-use regex")
        .find(source)
        .map(|matched| matched.start());

    match (suffix, prefix) {
        (Some(first), Some(second)) => Some(first.min(second)),
        (Some(position), None) | (None, Some(position)) => Some(position),
        _ => None,
    }
}

fn first_plain_assignment(source: &str, variable_name: &str) -> Option<usize> {
    let regex = Regex::new(&format!(
        r"\b{}\s*=",
        regex::escape(variable_name)
    ))
    .expect("plain assignment regex");

    for matched in regex.find_iter(source) {
        if source.as_bytes().get(matched.end()).copied() != Some(b'=') {
            return Some(matched.start());
        }
    }

    None
}

fn code_mask(source: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        LineComment,
        BlockComment,
        String,
    }

    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut state = State::Code;
    let mut index = 0;

    while index < bytes.len() {
        match state {
            State::Code => {
                if bytes[index] == b'/'
                    && index + 1 < bytes.len()
                    && bytes[index + 1] == b'/'
                {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::LineComment;
                    index += 2;
                } else if bytes[index] == b'/'
                    && index + 1 < bytes.len()
                    && bytes[index + 1] == b'*'
                {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::BlockComment;
                    index += 2;
                } else if bytes[index] == b'"' {
                    output[index] = b' ';
                    state = State::String;
                    index += 1;
                } else {
                    index += 1;
                }
            }
            State::LineComment => {
                if bytes[index] == b'\n' {
                    state = State::Code;
                } else {
                    output[index] = b' ';
                }
                index += 1;
            }
            State::BlockComment => {
                if bytes[index] == b'*'
                    && index + 1 < bytes.len()
                    && bytes[index + 1] == b'/'
                {
                    output[index] = b' ';
                    output[index + 1] = b' ';
                    state = State::Code;
                    index += 2;
                } else {
                    if bytes[index] != b'\n' {
                        output[index] = b' ';
                    }
                    index += 1;
                }
            }
            State::String => {
                if bytes[index] == b'\\' && index + 1 < bytes.len() {
                    output[index] = b' ';
                    if bytes[index + 1] != b'\n' {
                        output[index + 1] = b' ';
                    }
                    index += 2;
                } else if bytes[index] == b'"' {
                    output[index] = b' ';
                    state = State::Code;
                    index += 1;
                } else {
                    if bytes[index] != b'\n' {
                        output[index] = b' ';
                    }
                    index += 1;
                }
            }
        }
    }

    mask_preprocessor_lines(&mut output);
    String::from_utf8(output).expect("GLSL mask remains UTF-8")
}

fn mask_preprocessor_lines(bytes: &mut [u8]) {
    let mut line_start = 0;

    while line_start < bytes.len() {
        let line_end = bytes[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| line_start + offset)
            .unwrap_or(bytes.len());

        let first_non_whitespace = (line_start..line_end)
            .find(|index| bytes[*index] != b' ' && bytes[*index] != b'\t');

        if let Some(index) = first_non_whitespace {
            if bytes[index] == b'#' {
                bytes[line_start..line_end].fill(b' ');
            }
        }

        line_start = if line_end < bytes.len() {
            line_end + 1
        } else {
            bytes.len()
        };
    }
}

