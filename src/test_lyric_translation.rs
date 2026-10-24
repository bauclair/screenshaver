//! Terminal-only forward and reverse lyric translation experiment. No renderer/database writes.
use std::time::{Duration, Instant};

const MAX_CHUNK_BYTES: usize = 380; // Stay below MyMemory's small query limit.

pub fn run(target_locale: &str) -> Result<(), String> {
    let target = target_locale.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!("Invalid translation locale: {}", target_locale));
    }
    if target == "en" {
        return Err("Stage 3 expects a non-English target language (e.g. fr-FR)".into());
    }
    println!("Screenshaver Lyric Translation Test — Stage 3");
    println!("=============================================");
    println!("Requested target locale: {}", target_locale);
    let lines = crate::test_lyrics::retrieve_for_translation_test()?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyric-translation-test"))
        .build().map_err(|e| format!("HTTP client: {}", e))?;
    let active: Vec<usize> = lines.iter().enumerate()
        .filter_map(|(i, (_, text))| if text.trim().is_empty() { None } else { Some(i) })
        .collect();
    let mut translated = vec![String::new(); lines.len()];
    let mut providers = Vec::new();
    let started = Instant::now();
    let mut cursor = 0;
    while cursor < active.len() {
        let mut chunk = Vec::new();
        let mut bytes = 0;
        while cursor < active.len() {
            let idx = active[cursor];
            let next = lines[idx].1.len() + if chunk.is_empty() { 0 } else { 1 };
            if !chunk.is_empty() && bytes + next > MAX_CHUNK_BYTES { break; }
            if next > MAX_CHUNK_BYTES {
                return Err(format!("Lyric at {} exceeds safe translation request size", lines[idx].0));
            }
            chunk.push(idx);
            bytes += next;
            cursor += 1;
        }
        let source = chunk.iter().map(|&i| lines[i].1.as_str()).collect::<Vec<_>>().join("\n");
        let (result, provider) = translate_chunk(&client, &source, "en", &target, chunk.len())?;
        let output: Vec<&str> = result.split('\n').collect();
        if output.len() != chunk.len() {
            return Err(format!("Provider returned {} lines for {} source lines; refusing misalignment", output.len(), chunk.len()));
        }
        for (&idx, text) in chunk.iter().zip(output) {
            if text.trim().is_empty() {
                return Err(format!("Empty translation at {}", lines[idx].0));
            }
            translated[idx] = text.trim().to_string();
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
            let next = translated[idx].len() + if chunk.is_empty() { 0 } else { 1 };
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
        let source = chunk.iter().map(|&i| translated[i].as_str())
            .collect::<Vec<_>>().join("\n");
        match translate_chunk(&client, &source, &target, "en", chunk.len()) {
            Ok((result, provider)) => {
                for (&idx, text) in chunk.iter().zip(result.split('\n')) {
                    reversed[idx] = Some(text.trim().to_string());
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
    let reverse_elapsed = reverse_started.elapsed();

    println!("\nFORWARD TRANSLATION (en → {})", target);
    println!("=============================================");
    for (i, (stamp, original)) in lines.iter().enumerate() {
        println!("{} EN: {}", stamp, original);
        if !original.trim().is_empty() { println!("{} {}: {}", stamp, target.to_uppercase(), translated[i]); }
    }
    println!("\nOriginal entries: {} | Translated nonempty entries: {}", lines.len(), active.len());
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
    println!("\nReverse translated entries: {} / {}", reverse_count, active.len());
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
