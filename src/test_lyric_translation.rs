//! Terminal-only forward and reverse lyric translation experiment. No renderer/database writes.
use std::time::{Duration, Instant};
use mpris::{PlayerFinder, PlaybackStatus};

const MAX_CHUNK_BYTES: usize = 380; // Stay below MyMemory's small query limit.

// Stage 5 remains a terminal-only experiment. No production rendering or database writes.
// The request ID protects against late worker messages; production integration must
// additionally compare the current MPRIS track identity before publishing results.
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
const MAX_IN_FLIGHT_WORKERS: usize = 2;

struct ForwardResult {
    request_id: u64,
    translated: Vec<String>,
    providers: Vec<String>,
    elapsed: Duration,
}

fn translate_forward(
    lines: &[(String, String)], protected: &[ProtectedLine], target: &str,
    request_id: u64, cancelled: &AtomicBool,
) -> Result<ForwardResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyric-translation-test"))
        .build().map_err(|e| format!("HTTP client: {}", e))?;
    let active: Vec<usize> = protected.iter().enumerate()
        .filter_map(|(i, item)| if item.core.trim().is_empty() { None } else { Some(i) })
        .collect();
    let mut translated: Vec<String> = protected.iter()
        .map(|p| if p.is_vocal_only { p.original.clone() } else { String::new() })
        .collect();
    let mut providers = Vec::new();
    let started = Instant::now();
    let mut cursor = 0;
    while cursor < active.len() {
        if cancelled.load(Ordering::Acquire) { return Err("Translation superseded by another track".into()); }
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
        let (result, provider) = translate_chunk(&client, &source, "en", target, chunk.len())?;
        if cancelled.load(Ordering::Acquire) { return Err("Translation superseded by another track".into()); }
        let output: Vec<&str> = result.split('\n').collect();
        if output.len() != chunk.len() {
            return Err(format!("Provider returned {} lines for {} source lines; refusing misalignment", output.len(), chunk.len()));
        }
        for (&idx, text) in chunk.iter().zip(output) {
            if text.trim().is_empty() { return Err(format!("Empty translation at {}", lines[idx].0)); }
            translated[idx] = protected[idx].restore(text.trim());
        }
        providers.push(provider);
    }
    Ok(ForwardResult { request_id, translated, providers, elapsed: started.elapsed() })
}

// Snapshot both player ownership and track metadata. A changed or unreadable
// identity fails closed: the original lyrics remain available, but no stale
// translation is published. Do not compare playback position (it changes).
#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveTrack {
    bus_name: String,
    title: String,
    artists: Vec<String>,
    album: String,
    length_ms: Option<u128>,
}

fn live_track() -> Result<LiveTrack, String> {
    let finder = PlayerFinder::new().map_err(|e| format!("MPRIS connection: {}", e))?;
    let players = finder.find_all().map_err(|e| format!("MPRIS enumeration: {}", e))?;
    // Follow the same first-playing-player selection used by test_lyrics.
    for player in players {
        if !matches!(player.get_playback_status(), Ok(PlaybackStatus::Playing)) {
            continue;
        }
        let metadata = player.get_metadata().map_err(|e| format!("MPRIS metadata: {}", e))?;
        let title = metadata.title().ok_or("MPRIS title unavailable")?.to_owned();
        let artists = metadata.artists().unwrap_or_default().iter()
            .map(|s| (*s).to_owned()).collect();
        return Ok(LiveTrack {
            bus_name: player.bus_name().to_string(),
            title,
            artists,
            album: metadata.album_name().unwrap_or("").to_owned(),
            length_ms: metadata.length().map(|d| d.as_millis()),
        });
    }
    Err("No playing MPRIS player is available".into())
}

fn ensure_current(expected: &LiveTrack) -> Result<(), String> {
    let current = live_track()?;
    if &current == expected { Ok(()) } else {
        Err(format!("MPRIS track changed: expected {:?}; current {:?}", expected, current))
    }
}

fn matching_request(expected: u64, received: u64) -> bool { expected == received }

// Each worker decrements the shared count, including on errors or panics.
struct WorkerSlot(Arc<AtomicUsize>);
impl Drop for WorkerSlot {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::AcqRel); }
}

fn print_original(lines: &[(String, String)]) {
    println!("\nORIGINAL LYRICS READY (translation has not blocked presentation)");
    for (stamp, original) in lines { println!("{} EN: {}", stamp, original); }
}

fn print_translation(lines: &[(String, String)], protected: &[ProtectedLine],
                     target: &str, forward: ForwardResult) {
    let active_count = protected.iter().filter(|p| !p.core.trim().is_empty()).count();
    let protected_only = protected.iter().filter(|p| p.is_vocal_only).count();
    let protected_mixed = protected.iter().filter(|p| p.is_mixed).count();
    println!("\n[FOREGROUND] Validated translation published for request {}", forward.request_id);
    println!("\nFORWARD TRANSLATION (en → {})", target);
    println!("=============================================");
    for (i, (stamp, original)) in lines.iter().enumerate() {
        println!("{} EN: {}", stamp, original);
        if !original.trim().is_empty() {
            println!("{} {}: {}", stamp, target.to_uppercase(), forward.translated[i]);
        }
    }
    println!("\nOriginal entries: {} | Provider-translated entries: {}", lines.len(), active_count);
    println!("Protected vocalization-only entries: {} | Mixed entries: {}", protected_only, protected_mixed);
    println!("Forward worker elapsed: {:.3} seconds", forward.elapsed.as_secs_f64());
    println!("Providers per chunk: {}", forward.providers.join(", "));
    println!("Timestamp alignment: PASS (exact positional mapping and line count)");
    println!("Track-change safety: PASS (live identity checked before publication)");
}

pub fn run(target_locale: &str) -> Result<(), String> {
    let target = target_locale.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    if target.is_empty() || !target.chars().all(|c| c.is_ascii_alphabetic()) || target == "en" {
        return Err(format!("Stage 5C requires a non-English target locale: {}", target_locale));
    }
    println!("Screenshaver Lyric Translation Test — Stage 5C (continuous track handling)");
    println!("Requested target locale: {}", target_locale);
    println!("Press Ctrl+C to stop. Skip songs while translation is in progress.");
    println!("Reverse translation is disabled in continuous mode.");

    let in_flight = Arc::new(AtomicUsize::new(0));
    let mut last_track: Option<LiveTrack> = None;
    // A generation is incremented for every newly detected track, even when
    // retrieval fails. Never publish a worker result across generations.
    let mut generation: u64 = 0;
    loop {
        let current = match live_track() {
            Ok(track) => track,
            Err(error) => {
                if last_track.take().is_some() {
                    generation += 1;
                    println!("[FOREGROUND] Player unavailable: {}; pending translation invalidated", error);
                }
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
        };
        if last_track.as_ref() == Some(&current) {
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }
        generation += 1;
        last_track = Some(current.clone());
        println!("\n[FOREGROUND] Track generation {}: {} — {}", generation,
                 current.artists.join(", "), current.title);
        let lines = match crate::test_lyrics::retrieve_for_translation_test() {
            Ok(lines) => lines,
            Err(error) => {
                println!("[FOREGROUND] Lyrics unavailable: {}. Watching for next track.", error);
                continue;
            }
        };
        if ensure_current(&current).is_err() {
            println!("[FOREGROUND] Track changed during lyrics retrieval; discarding lyrics.");
            // Next iteration will detect the newly active track.
            continue;
        }
        print_original(&lines);
        let protected: Vec<ProtectedLine> = lines.iter()
            .map(|(_, text)| protect_vocalizations(text)).collect();
        // Limit detached network workers. An old request can finish its current
        // blocking HTTP call, but cancellation stops it before the next chunk.
        while in_flight.load(Ordering::Acquire) >= MAX_IN_FLIGHT_WORKERS {
            if ensure_current(&current).is_err() { break; }
            println!("[FOREGROUND] Waiting for an obsolete HTTP worker slot...");
            std::thread::sleep(Duration::from_millis(250));
        }
        if ensure_current(&current).is_err() {
            println!("[FOREGROUND] Track changed before worker start; discarding request.");
            continue;
        }
        let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker_count = Arc::clone(&in_flight);
        let worker_lines = lines.clone();
        let worker_protected = protected.clone();
        let worker_target = target.clone();
        in_flight.fetch_add(1, Ordering::AcqRel);
        let spawn = std::thread::Builder::new().name("lyric-translation-test".into())
            .spawn(move || {
                let _slot = WorkerSlot(worker_count);
                let result = translate_forward(&worker_lines, &worker_protected,
                                               &worker_target, request_id, &worker_cancelled);
                let _ = tx.send(result);
            });
        if let Err(error) = spawn {
            in_flight.fetch_sub(1, Ordering::AcqRel);
            println!("[FOREGROUND] Worker launch failed: {}", error);
            continue;
        }
        println!("[FOREGROUND] Translation request {} started; foreground remains responsive.", request_id);
        let started = Instant::now();
        loop {
            // Check BEFORE reading results, and AGAIN before publication.
            if ensure_current(&current).is_err() {
                cancelled.store(true, Ordering::Release);
                println!("[FOREGROUND] Track change detected; request {} discarded.", request_id);
                println!("[FOREGROUND] Stale-result protection: PASS.");
                break;
            }
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(Ok(result)) => {
                    if !matching_request(request_id, result.request_id)
                        || ensure_current(&current).is_err() {
                        println!("[FOREGROUND] Stale worker result rejected: {}", result.request_id);
                    } else {
                        println!("[FOREGROUND] Worker completed after {:.3}s", started.elapsed().as_secs_f64());
                        print_translation(&lines, &protected, &target, result);
                    }
                    break;
                }
                Ok(Err(error)) => {
                    println!("[FOREGROUND] Translation unavailable: {}. Original lyrics retained.", error);
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    println!("[FOREGROUND] Worker disconnected. Original lyrics retained.");
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    println!("[FOREGROUND] Responsive at {:.2}s; original lyrics available",
                             started.elapsed().as_secs_f64());
                }
            }
        }
        // If translation succeeded, wait for the next song instead of
        // translating this same track repeatedly.
    }
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
#[derive(Clone)]
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

#[cfg(test)]
mod stage5_tests {
    use super::*;
    #[test]
    fn rejects_stale_worker_request() {
        assert!(matching_request(17, 17));
        assert!(!matching_request(18, 17));
        let a = LiveTrack { bus_name: "org.mpris.MediaPlayer2.kew".into(), title: "A".into(), artists: vec!["Artist".into()], album: "Album".into(), length_ms: Some(1234) };
        let mut b = a.clone();
        assert_eq!(a, b);
        b.title = "B".into();
        assert_ne!(a, b);
    }
}
