//! Terminal-only forward and reverse lyric translation experiment. No renderer/database writes.
use std::time::{Duration, Instant};

const MAX_CHUNK_BYTES: usize = 380; // Stay below MyMemory's small query limit.

pub fn run(target_locale: &str) -> Result<(), String> {
    let target = target_locale.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("Invalid translation locale: {}", target_locale));
    }
    if target == "en" {
        return Err("Stage 4 expects a non-English target language (e.g. fr-FR)".into());
    }
    println!("Screenshaver Lyric Translation Test — Stage 4");
    println!("=============================================");
    println!("Requested target locale: {}", target_locale);
    let lines = crate::test_lyrics::retrieve_for_translation_test()?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyric-translation-test"))
        .build().map_err(|e| format!("HTTP client: {}", e))?;
    let protected: Vec<ProtectedLine> = lines.iter()
        .map(|(_, text)| protect_vocalizations(text))
        .collect();
    let active: Vec<usize> = protected.iter().enumerate()
        .filter_map(|(i, item)| if item.core.trim().is_empty() { None } else { Some(i) })
        .collect();
    let protected_only = protected.iter().filter(|p| p.is_vocal_only).count();
    let protected_mixed = protected.iter().filter(|p| p.is_mixed).count();
    let mut translated: Vec<String> = protected.iter()
        .map(|p| if p.is_vocal_only { p.original.clone() } else { String::new() })
        .collect();
    let mut providers = Vec::new();
    let started = Instant::now();
    let mut cursor = 0;
    while cursor < active.len() {
        let mut chunk = Vec::new();
        let mut bytes = 0;
        while cursor < active.len() {
            let idx = active[cursor];
            let next = protected[idx].core.len() + if chunk.is_empty() { 0 } else { 1 };
            if !chunk.is_empty() && bytes + next > MAX_CHUNK_BYTES { break; }
            if next > MAX_CHUNK_BYTES {
                return Err(format!("Lyric at {} exceeds safe translation request size", lines[idx].0));
            }
            chunk.push(idx);
            bytes += next;
            cursor += 1;
        }
        let source = chunk.iter().map(|&i| protected[i].core.as_str()).collect::<Vec<_>>().join("\n");
        let (result, provider) = translate_chunk(&client, &source, "en", &target, chunk.len())?;
        let output: Vec<&str> = result.split('\n').collect();
        if output.len() != chunk.len() {
            return Err(format!("Provider returned {} lines for {} source lines; refusing misalignment", output.len(), chunk.len()));
        }
        for (&idx, text) in chunk.iter().zip(output) {
            if text.trim().is_empty() {
                return Err(format!("Empty translation at {}", lines[idx].0));
            }
            translated[idx] = protected[idx].restore(text.trim());
        }
        providers.push(provider);
    }
    let forward_elapsed = started.elapsed();

    // Reverse translation is diagnostic only. Never alter the validated forward
    // translation or its original timestamps if a reverse provider fails.
    let reverse_started = Instant::now();
    let mut reversed: Vec<Option<String>> = vec![None; lines.len()];
    let mut reverse_providers = Vec::new();
    let mut reverse_errors = Vec::new();
    let mut reverse_cursor = 0;
    while reverse_cursor < active.len() {
        let mut chunk = Vec::new();
        let mut bytes = 0;
        while reverse_cursor < active.len() {
            let idx = active[reverse_cursor];
            let next = extract_translated_core(&translated[idx], &protected[idx]).len() + if chunk.is_empty() { 0 } else { 1 };
            if !chunk.is_empty() && bytes + next > MAX_CHUNK_BYTES { break; }
            if next > MAX_CHUNK_BYTES {
                reverse_errors.push(format!(
                    "{}: translated lyric exceeds safe reverse request size",
                    lines[idx].0
                ));
                reverse_cursor += 1;
                break;
            }
            chunk.push(idx);
            bytes += next;
            reverse_cursor += 1;
        }
        if chunk.is_empty() { continue; }
        let reverse_inputs: Vec<String> = chunk.iter()
            .map(|&i| extract_translated_core(&translated[i], &protected[i]).to_string())
            .collect();
        let source = reverse_inputs.join("\n");
        match translate_chunk(&client, &source, &target, "en", chunk.len()) {
            Ok((result, provider)) => {
                for (&idx, text) in chunk.iter().zip(result.split('\n')) {
                    reversed[idx] = Some(protected[idx].restore(text.trim()));
                }
                reverse_providers.push(provider);
            }
            Err(error) => {
                reverse_errors.push(format!(
                    "Reverse chunk beginning at {}: {}", lines[chunk[0]].0, error
                ));
            }
        }
    }
    for (i, item) in protected.iter().enumerate() {
        if item.is_vocal_only { reversed[i] = Some(item.original.clone()); }
    }
    let reverse_elapsed = reverse_started.elapsed();

    println!("\nFORWARD TRANSLATION (en → {})", target);
    println!("=============================================");
    for (i, (stamp, original)) in lines.iter().enumerate() {
        println!("{} EN: {}", stamp, original);
        if !original.trim().is_empty() { println!("{} {}: {}", stamp, target.to_uppercase(), translated[i]); }
    }
    println!("\nOriginal entries: {} | Provider-translated entries: {}", lines.len(), active.len());
    println!("Protected vocalization-only entries: {} | Mixed entries with protected edge vocalizations: {}", protected_only, protected_mixed);
    println!("Already-target-language detection: not applied (conservative; no language classifier)");
    println!("Forward translation elapsed: {:.3} seconds", forward_elapsed.as_secs_f64());
    println!("Providers per chunk: {}", providers.join(", "));
    println!("Timestamp alignment: PASS (positional mapping, exact line-count validation)");

    println!("\nREVERSE TRANSLATION DIAGNOSTIC ({} → en)", target);
    println!("=================================================");
    for (i, (stamp, original)) in lines.iter().enumerate() {
        if original.trim().is_empty() { continue; }
        println!("{} ORIGINAL EN: {}", stamp, original);
        println!("{} FORWARD {}: {}", stamp, target.to_uppercase(), translated[i]);
        match &reversed[i] {
            Some(text) => println!("{} REVERSE EN: {}", stamp, text),
            None => println!("{} REVERSE EN: <unavailable>", stamp),
        }
    }
    let reverse_count = reversed.iter().filter(|v| v.is_some()).count();
    println!("\nReverse available entries: {} / {} (includes {} protected-only entries)", reverse_count, active.len() + protected_only, protected_only);
    println!("Reverse translation elapsed: {:.3} seconds", reverse_elapsed.as_secs_f64());
    println!("Reverse providers per chunk: {}", if reverse_providers.is_empty() {
        "<none>".to_string()
    } else { reverse_providers.join(", ") });
    if reverse_errors.is_empty() {
        println!("Reverse timestamp alignment: PASS (positional mapping, exact line-count validation)");
    } else {
        println!("Reverse timestamp alignment: PARTIAL ({} chunk errors)", reverse_errors.len());
        for error in &reverse_errors { eprintln!("[REVERSE TRANSLATION] {}", error); }
    }
    println!("NOTE: Back-translation is a qualitative aid, not an automatic accuracy score.");
    Ok(())
}

fn translate_chunk(client: &reqwest::blocking::Client, source: &str, from: &str, to: &str, expected: usize) -> Result<(String, String), String> {
    let mut errors = Vec::new();
    match mymemory(client, source, from, to) {
        Ok(s) if valid_line_count(&s, expected) => return Ok((s, "MyMemory".into())),
        Ok(s) => errors.push(format!("MyMemory line count mismatch: {} vs {}", s.split('\n').count(), expected)),
        Err(e) => errors.push(format!("MyMemory: {}", e)),
    }
    std::thread::sleep(Duration::from_millis(250));
    match google(client, source, from, to) {
        Ok(s) if valid_line_count(&s, expected) => Ok((s, "Google reference endpoint".into())),
        Ok(s) => Err(format!("Translation line alignment failed: {}; Google returned {} vs {} lines", errors.join("; "), s.split('\n').count(), expected)),
        Err(e) => Err(format!("Translation providers failed: {}; Google: {}", errors.join("; "), e)),
    }
}

fn valid_line_count(text: &str, count: usize) -> bool {
    text.split('\n').count() == count && text.split('\n').all(|line| !line.trim().is_empty())
}

fn mymemory(client: &reqwest::blocking::Client, source: &str, from: &str, to: &str) -> Result<String, String> {
    let pair = format!("{}|{}", from, to);
    let response = client.get("https://api.mymemory.translated.net/get")
        .query(&[("q", source), ("langpair", pair.as_str())])
        .send().map_err(|e| e.to_string())?;
    if !response.status().is_success() { return Err(format!("HTTP {}", response.status())); }
    let data: serde_json::Value = response.json().map_err(|e| e.to_string())?;
    let status = data.get("responseStatus").and_then(|v| v.as_i64()).unwrap_or(0);
    if status != 200 { return Err(format!("provider status {}: {}", status, data["responseDetails"])); }
    data.pointer("/responseData/translatedText").and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty()).map(str::to_owned)
        .ok_or_else(|| "missing translatedText".into())
}

fn google(client: &reqwest::blocking::Client, source: &str, from: &str, to: &str) -> Result<String, String> {
    let response = client.get("https://translate.googleapis.com/translate_a/single")
        .query(&[("client", "gtx"), ("sl", from), ("tl", to), ("dt", "t"), ("q", source)])
        .send().map_err(|e| e.to_string())?;
    if !response.status().is_success() { return Err(format!("HTTP {}", response.status())); }
    let data: serde_json::Value = response.json().map_err(|e| e.to_string())?;
    let segments = data.get(0).and_then(|v| v.as_array()).ok_or("missing translation segments")?;
    let output: String = segments.iter().filter_map(|segment| segment.get(0).and_then(|v| v.as_str())).collect();
    if output.trim().is_empty() { Err("empty translation".into()) } else { Ok(output) }
}


// Stage 4: deliberately conservative. Only standalone vocalizations and vocal
// tokens at the beginning/end of a line are protected. Do not rewrite internal
// phrases or attempt to guess the language of short lyric fragments.
struct ProtectedLine {
    original: String,
    prefix: String,
    core: String,
    suffix: String,
    is_vocal_only: bool,
    is_mixed: bool,
}

impl ProtectedLine {
    fn restore(&self, translated_core: &str) -> String {
        format!("{}{}{}", self.prefix, translated_core, self.suffix)
    }
}

fn vocal_token(token: &str) -> bool {
    let cleaned = token.trim_matches(|c: char| !c.is_alphabetic() && c != '-')
        .to_ascii_lowercase();
    if cleaned.is_empty() { return false; }
    // Hyphenated repetitions count only when *every* syllable is recognized.
    cleaned.split('-').all(|part| matches!(part,
        "na" | "la" | "da" | "doo" | "ooh" | "oh" | "ah" | "uh" |
        "whoa" | "woo" | "woah" | "mm" | "mmm" | "hmm" | "ch"))
}

fn safe_mixed_vocal_token(token: &str) -> bool {
    let cleaned = token.trim_matches(|c: char| !c.is_alphabetic() && c != '-')
        .to_ascii_lowercase();
    vocal_token(token) && !matches!(cleaned.as_str(), "oh" | "ah" | "uh" | "da" | "la" | "na")
}

fn protect_vocalizations(text: &str) -> ProtectedLine {
    // Use whitespace-delimited byte ranges; slicing preserves original spelling.
    let mut tokens = Vec::new();
    let mut start = None;
    for (pos, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(begin) = start.take() { tokens.push((begin, pos)); }
        } else if start.is_none() { start = Some(pos); }
    }
    if let Some(begin) = start { tokens.push((begin, text.len())); }
    let mut left = 0;
    while left < tokens.len() && vocal_token(&text[tokens[left].0..tokens[left].1]) { left += 1; }
    let mut right = tokens.len();
    while right > left && vocal_token(&text[tokens[right - 1].0..tokens[right - 1].1]) { right -= 1; }
    let only = !tokens.is_empty() && left == tokens.len();
    // Single isolated words such as "oh" can be meaningful; preserve only
    // multi-token vocal lines or explicit nonlexical spellings on their own.
    let single_nonlexical = tokens.len() == 1 && matches!(
        text[tokens[0].0..tokens[0].1].trim_matches(|c: char| !c.is_alphabetic()).to_ascii_lowercase().as_str(),
        "ooh" | "doo" | "mmm" | "hmm" | "whoa" | "woah" | "la-la" | "na-na" | "da-da");
    if only && (tokens.len() >= 2 || single_nonlexical) {
        return ProtectedLine { original: text.into(), prefix: text.into(), core: String::new(),
            suffix: String::new(), is_vocal_only: true, is_mixed: false };
    }
    // Mixed-line protection is stricter than whole-line recognition:
    // do not strip a meaningful introductory "Oh" or similar word.
    left = 0;
    while left < tokens.len() && safe_mixed_vocal_token(&text[tokens[left].0..tokens[left].1]) { left += 1; }
    right = tokens.len();
    while right > left && safe_mixed_vocal_token(&text[tokens[right - 1].0..tokens[right - 1].1]) { right -= 1; }
    // A trailing ", oh" is a standalone sung interjection, but "oh" within
    // ordinary prose can carry meaning. Protect only this narrow punctuation-
    // delimited suffix; leave introductory and unseparated "oh" to translation.
    if right == tokens.len() && tokens.len() >= 2 {
        let last = &text[tokens[right - 1].0..tokens[right - 1].1];
        let previous = &text[tokens[right - 2].0..tokens[right - 2].1];
        if last.trim_matches(|c: char| !c.is_alphabetic()).eq_ignore_ascii_case("oh")
            && previous.ends_with(',')
        {
            right -= 1;
        }
    }
    if left == tokens.len() { left = 0; }
    let core_start = if left > 0 { tokens[left].0 } else { 0 };
    let core_end = if right < tokens.len() { tokens[right - 1].1 } else { text.len() };
    let mixed = (left > 0 || right < tokens.len()) && core_start < core_end;
    ProtectedLine { original: text.into(), prefix: text[..core_start].into(),
        core: text[core_start..core_end].into(), suffix: text[core_end..].into(),
        is_vocal_only: false, is_mixed: mixed }
}

fn extract_translated_core<'a>(translated: &'a str, item: &ProtectedLine) -> &'a str {
    translated.strip_prefix(&item.prefix).and_then(|s| s.strip_suffix(&item.suffix))
        .unwrap_or(translated)
}

#[cfg(test)]
mod stage4_tests {
    use super::*;
    #[test]
    fn protects_repetitions_and_mixed_edges() {
        for s in ["Na na na na", "Ooh! Ch-ch!", "La-la", "Ooh!"] {
            let p = protect_vocalizations(s);
            assert!(p.is_vocal_only, "{s}");
            assert_eq!(p.original, p.restore(""));
        }
        let p = protect_vocalizations("La-la how the life goes on");
        assert!(p.is_mixed);
        assert_eq!(p.core, "how the life goes on");
        assert_eq!(p.restore("comment la vie continue"), "La-la comment la vie continue");
        let p = protect_vocalizations("Drinkin' it down, oh");
        assert_eq!(p.core, "Drinkin' it down,");
        assert_eq!(p.restore("Bois-le,"), "Bois-le, oh");
        assert!(!protect_vocalizations("Oh no, my love's at an end").is_vocal_only);
        assert_eq!(protect_vocalizations("Oh no, my love's at an end").core, "Oh no, my love's at an end");
        assert_eq!(protect_vocalizations("I said oh no").core, "I said oh no");
    }
}
