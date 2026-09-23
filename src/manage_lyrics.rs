use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const LRCLIB_GET_URL: &str = "https://lrclib.net/api/get";
const LRCMUX_GET_URL: &str = "https://api.lrcmux.dev/get";
const POSITION_POLL_INTERVAL: Duration = Duration::from_millis(50);
const METADATA_POLL_INTERVAL: Duration = Duration::from_millis(500);
const PLAYER_SCAN_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LyricsState {
    pub previous: Option<String>,
    pub current: Option<String>,
    pub next: Option<String>,
}

pub type SharedLyricsState = Arc<Mutex<LyricsState>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocalTimingState {
    pub available: bool,
    pub active: bool,
    pub current_line: Option<String>,
}

pub type SharedVocalTimingState = Arc<Mutex<VocalTimingState>>;

pub struct LyricsManager {
    state: SharedLyricsState,
    vocal_timing_state: SharedVocalTimingState,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LyricsManager {
    pub fn start() -> Result<Self, String> {
        // Verify that the session D-Bus is reachable before reporting a
        // successfully started manager. The worker owns all later MPRIS access.
        PlayerFinder::new().map_err(|error| {
            format!(
                "Unable to connect to the session D-Bus for MPRIS discovery: {}",
                error
            )
        })?;

        let state = Arc::new(Mutex::new(LyricsState::default()));
        let vocal_timing_state = Arc::new(Mutex::new(VocalTimingState::default()));
        let running = Arc::new(AtomicBool::new(true));
        let worker_state = Arc::clone(&state);
        let worker_vocal_timing_state = Arc::clone(&vocal_timing_state);
        let worker_running = Arc::clone(&running);

        let worker = thread::Builder::new()
            .name("screenshaver-lyrics".to_string())
            .spawn(move || {
                run_worker(worker_state, worker_vocal_timing_state, worker_running);
            })
            .map_err(|error| format!("Unable to start lyrics worker: {}", error))?;

        log_information("[LYRICS] Production lyrics manager started");

        Ok(Self {
            state,
            vocal_timing_state,
            running,
            worker: Some(worker),
        })
    }

    pub fn shared_state(&self) -> SharedLyricsState {
        Arc::clone(&self.state)
    }

    pub fn current_state(&self) -> LyricsState {
        self.state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    pub fn shared_vocal_timing_state(&self) -> SharedVocalTimingState {
        Arc::clone(&self.vocal_timing_state)
    }

    pub fn current_vocal_timing_state(&self) -> VocalTimingState {
        self.vocal_timing_state
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default()
    }
}

impl Drop for LyricsManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                log_warning("[LYRICS] Lyrics worker terminated unexpectedly");
            }
        }

        log_information("[LYRICS] Production lyrics manager stopped");
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibResponse {
    id: i64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxResponse {
    track: LrcmuxTrack,
    meta: LrcmuxMeta,
    lines: Vec<LrcmuxLine>,
}

#[derive(Debug, Deserialize)]
struct LrcmuxTrack {
    title: String,
    artist: String,
    album: String,
    duration: u64,
}

#[derive(Debug, Deserialize)]
struct LrcmuxMeta {
    source: LrcmuxSource,
    level: String,
}

#[derive(Debug, Deserialize)]
struct LrcmuxSource {
    id: String,
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct LrcmuxLine {
    text: String,
    start: u64,
    end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackIdentity {
    title: String,
    artist: String,
    album: String,
    duration_milliseconds: u128,
}

#[derive(Debug, Clone)]
struct TrackInformation {
    identity: TrackIdentity,
    duration: Duration,
}

#[derive(Debug, Clone)]
struct SynchronizedLine {
    timestamp: Duration,
    text: String,
}

#[derive(Debug, Clone)]
struct VocalInterval {
    start: Duration,
    end: Duration,
    text: String,
}

fn run_worker(
    state: Arc<Mutex<LyricsState>>,
    vocal_timing_state: Arc<Mutex<VocalTimingState>>,
    running: Arc<AtomicBool>,
) {
    let mut last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
    let mut last_metadata_poll = Instant::now() - METADATA_POLL_INTERVAL;
    let mut last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
    let mut active_player: Option<Player> = None;
    let mut current_track: Option<TrackInformation> = None;
    let mut synchronized_lines: Vec<SynchronizedLine> = Vec::new();
    let mut vocal_intervals: Vec<VocalInterval> = Vec::new();
    let mut last_line_index: Option<usize> = None;
    let mut last_vocal_interval_index: Option<usize> = None;

    while running.load(Ordering::SeqCst) {
        if let Some(player) = active_player.as_ref() {
            match player.get_playback_status() {
                Ok(PlaybackStatus::Playing) => {}
                Ok(status) => {
                    log_information(&format!(
                        "[LYRICS] Active player '{}' is now {:?}; searching for another playing player",
                        player.identity(), status
                    ));
                    active_player = None;
                    current_track = None;
                    synchronized_lines.clear();
                    vocal_intervals.clear();
                    last_line_index = None;
                    last_vocal_interval_index = None;
                    publish_state(&state, LyricsState::default());
                    publish_vocal_timing_state(&vocal_timing_state, VocalTimingState::default());
                    last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
                }
                Err(error) => {
                    log_warning(&format!(
                        "[LYRICS] Active player '{}' is no longer readable: {}",
                        player.identity(), error
                    ));
                    active_player = None;
                    current_track = None;
                    synchronized_lines.clear();
                    vocal_intervals.clear();
                    last_line_index = None;
                    last_vocal_interval_index = None;
                    publish_state(&state, LyricsState::default());
                    publish_vocal_timing_state(&vocal_timing_state, VocalTimingState::default());
                    last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
                }
            }
        }

        if active_player.is_none() && last_player_scan.elapsed() >= PLAYER_SCAN_INTERVAL {
            last_player_scan = Instant::now();

            match find_playing_player() {
                Ok(Some(player)) => match load_player_track(&player) {
                    Ok((track, lines, intervals)) => {
                        log_information(&format!(
                            "[LYRICS] Following MPRIS player '{}' for '{}' by '{}'",
                            player.identity(), track.identity.title, track.identity.artist
                        ));
                        current_track = Some(track);
                        synchronized_lines = lines;
                        vocal_intervals = intervals;
                        last_line_index = None;
                        last_vocal_interval_index = None;
                        active_player = Some(player);
                        last_metadata_poll = Instant::now();
                        last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
                        publish_state(&state, LyricsState::default());
                        publish_vocal_timing_state(
                            &vocal_timing_state,
                            VocalTimingState {
                                available: !vocal_intervals.is_empty(),
                                active: false,
                                current_line: None,
                            },
                        );
                    }
                    Err(error) => {
                        log_warning(&format!("[LYRICS] Playing player could not be used: {}", error));
                    }
                },
                Ok(None) => {}
                Err(error) => log_warning(&format!("[LYRICS] MPRIS discovery failed: {}", error)),
            }
        }

        if last_metadata_poll.elapsed() >= METADATA_POLL_INTERVAL {
            last_metadata_poll = Instant::now();

            if let (Some(player), Some(track)) =
                (active_player.as_ref(), current_track.as_mut())
            {
                match read_track_information(player) {
                    Ok(observed_track) => {
                        if observed_track.identity != track.identity {
                            log_information(&format!(
                                "[LYRICS] Track change detected on '{}': '{}' by '{}'",
                                player.identity(),
                                observed_track.identity.title,
                                observed_track.identity.artist
                            ));

                            *track = observed_track;
                            synchronized_lines = match retrieve_synchronized_lines(track) {
                                Ok(lines) => lines,
                                Err(error) => {
                                    log_warning(&format!("[LYRICS] {}", error));
                                    Vec::new()
                                }
                            };
                            vocal_intervals = match retrieve_lrcmux_vocal_intervals(track) {
                                Ok(intervals) => intervals,
                                Err(error) => {
                                    log_warning(&format!(
                                        "[AUDIO MOTION] LRCMUX vocal timing unavailable: {}",
                                        error
                                    ));
                                    Vec::new()
                                }
                            };
                            last_line_index = None;
                            last_vocal_interval_index = None;
                            publish_state(&state, LyricsState::default());
                            publish_vocal_timing_state(
                                &vocal_timing_state,
                                VocalTimingState {
                                    available: !vocal_intervals.is_empty(),
                                    active: false,
                                    current_line: None,
                                },
                            );
                        }
                    }
                    Err(error) => log_warning(&format!(
                        "[LYRICS] Unable to refresh track metadata from '{}': {}",
                        player.identity(), error
                    )),
                }
            }
        }

        if last_position_poll.elapsed() >= POSITION_POLL_INTERVAL {
            last_position_poll = Instant::now();

            if let Some(player) = active_player.as_ref() {
                if let Ok(position) = player.get_position() {
                    let active = find_active_line(&synchronized_lines, position);
                    if active != last_line_index {
                        last_line_index = active;
                        publish_line_state(&state, &synchronized_lines, active);
                    }

                    let vocal_active = find_active_vocal_interval(&vocal_intervals, position);
                    if vocal_active != last_vocal_interval_index {
                        last_vocal_interval_index = vocal_active;
                        publish_vocal_interval_state(
                            &vocal_timing_state,
                            &vocal_intervals,
                            vocal_active,
                        );
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    publish_state(&state, LyricsState::default());
    publish_vocal_timing_state(&vocal_timing_state, VocalTimingState::default());
}

fn publish_line_state(
    state: &Arc<Mutex<LyricsState>>,
    lines: &[SynchronizedLine],
    active: Option<usize>,
) {
    let Some(active) = active else {
        publish_state(state, LyricsState::default());
        return;
    };

    let next_state = LyricsState {
        previous: active
            .checked_sub(1)
            .and_then(|index| lines.get(index))
            .map(|line| line.text.clone()),
        current: lines.get(active).map(|line| line.text.clone()),
        next: lines.get(active + 1).map(|line| line.text.clone()),
    };

    publish_state(state, next_state);
}

fn publish_state(state: &Arc<Mutex<LyricsState>>, next_state: LyricsState) {
    if let Ok(mut state) = state.lock() {
        *state = next_state;
    }
}

fn publish_vocal_timing_state(
    state: &Arc<Mutex<VocalTimingState>>,
    next_state: VocalTimingState,
) {
    if let Ok(mut state) = state.lock() {
        *state = next_state;
    }
}

fn publish_vocal_interval_state(
    state: &Arc<Mutex<VocalTimingState>>,
    intervals: &[VocalInterval],
    active: Option<usize>,
) {
    let current_line =
        active
            .and_then(|index| intervals.get(index))
            .map(|interval| interval.text.clone());

    publish_vocal_timing_state(
        state,
        VocalTimingState {
            available: !intervals.is_empty(),
            active: active.is_some(),
            current_line,
        },
    );
}

fn find_playing_player() -> Result<Option<Player>, String> {
    let finder = PlayerFinder::new().map_err(|error| {
        format!(
            "Unable to connect to the session D-Bus for MPRIS discovery: {}",
            error
        )
    })?;

    let players = finder
        .find_all()
        .map_err(|error| format!("Unable to enumerate MPRIS media players: {}", error))?;

    for player in players {
        match player.get_playback_status() {
            Ok(PlaybackStatus::Playing) => return Ok(Some(player)),
            Ok(_) => {}
            Err(error) => log_warning(&format!(
                "[LYRICS] Unable to read playback status from '{}': {}",
                player.identity(), error
            )),
        }
    }

    Ok(None)
}

fn load_player_track(
    player: &Player,
) -> Result<(TrackInformation, Vec<SynchronizedLine>, Vec<VocalInterval>), String> {
    let track = read_track_information(player)?;
    let lines = match retrieve_synchronized_lines(&track) {
        Ok(lines) => lines,
        Err(error) => {
            log_warning(&format!("[LYRICS] {}", error));
            Vec::new()
        }
    };

    // Audio Motion deliberately uses LRCMUX only.  Its explicit start/end
    // intervals let the test gate vocal emphasis without guessing where a
    // lyric line stops.  Failure here does not affect ordinary lyric display.
    let vocal_intervals = match retrieve_lrcmux_vocal_intervals(&track) {
        Ok(intervals) => intervals,
        Err(error) => {
            log_warning(&format!(
                "[AUDIO MOTION] LRCMUX vocal timing unavailable: {}",
                error
            ));
            Vec::new()
        }
    };

    Ok((track, lines, vocal_intervals))
}

fn read_track_information(player: &Player) -> Result<TrackInformation, String> {
    let metadata = player.get_metadata().map_err(|error| {
        format!(
            "Unable to obtain metadata from {}: {}",
            player.identity(), error
        )
    })?;

    let title = metadata
        .title()
        .ok_or_else(|| "The selected MPRIS player did not provide a track title.".to_string())?
        .to_string();

    let artist = metadata
        .artists()
        .and_then(|artists| artists.first().copied())
        .ok_or_else(|| "The selected MPRIS player did not provide an artist.".to_string())?
        .to_string();

    let album = metadata.album_name().unwrap_or("").to_string();
    let duration = metadata
        .length()
        .ok_or_else(|| "The selected MPRIS player did not provide a track duration.".to_string())?;

    Ok(TrackInformation {
        identity: TrackIdentity {
            title,
            artist,
            album,
            duration_milliseconds: duration.as_millis(),
        },
        duration,
    })
}

fn retrieve_synchronized_lines(track: &TrackInformation) -> Result<Vec<SynchronizedLine>, String> {
    match retrieve_lrclib_synchronized_lines(track) {
        Ok(lines) => {
            log_information("[LYRICS] Lyrics provider: LRCLIB");
            Ok(lines)
        }
        Err(lrclib_error) => {
            log_warning(&format!("[LYRICS] LRCLIB lookup failed: {}", lrclib_error));
            match retrieve_lrcmux_synchronized_lines(track) {
                Ok(lines) => {
                    log_information("[LYRICS] Lyrics provider: LRCMUX");
                    Ok(lines)
                }
                Err(lrcmux_error) => Err(format!(
                    "No synchronized lyrics available. LRCLIB: {} LRCMUX: {}",
                    lrclib_error, lrcmux_error
                )),
            }
        }
    }
}

fn retrieve_lrclib_synchronized_lines(
    track: &TrackInformation,
) -> Result<Vec<SynchronizedLine>, String> {
    let result = query_lrclib(
        &track.identity.title,
        &track.identity.artist,
        &track.identity.album,
        track.duration,
    )?;

    let synced_lyrics = result
        .synced_lyrics
        .as_deref()
        .filter(|lyrics| has_text(Some(lyrics)))
        .ok_or_else(|| {
            format!(
                "LRCLIB returned no synchronized lyrics for '{}' by '{}'.",
                track.identity.title, track.identity.artist
            )
        })?;

    let lines = parse_synchronized_lyrics(synced_lyrics)?;
    log_information(&format!(
        "[LYRICS] LRCLIB match id={} track='{}' artist='{}' album='{}' duration={:.3}s instrumental={} plain={} synchronized_lines={}",
        result.id,
        result.track_name,
        result.artist_name,
        result.album_name,
        result.duration,
        result.instrumental,
        has_text(result.plain_lyrics.as_deref()),
        lines.len()
    ));
    Ok(lines)
}

fn retrieve_lrcmux_synchronized_lines(
    track: &TrackInformation,
) -> Result<Vec<SynchronizedLine>, String> {
    let result = query_lrcmux(
        &track.identity.title,
        &track.identity.artist,
        &track.identity.album,
        track.duration,
    )?;

    if result.lines.is_empty() {
        return Err(format!(
            "LRCMUX returned no synchronized lyrics for '{}' by '{}'.",
            track.identity.title, track.identity.artist
        ));
    }

    log_information(&format!(
        "[LYRICS] LRCMUX match track='{}' artist='{}' album='{}' duration={}s source='{}' ({}) source_url='{}' synchronization='{}' lines={}",
        result.track.title,
        result.track.artist,
        result.track.album,
        result.track.duration,
        result.meta.source.name,
        result.meta.source.id,
        result.meta.source.url,
        result.meta.level,
        result.lines.len()
    ));

    let mut synchronized_lines = Vec::with_capacity(result.lines.len());
    for line in result.lines {
        synchronized_lines.push(SynchronizedLine {
            timestamp: Duration::from_millis(line.start),
            text: line.text,
        });
    }
    synchronized_lines.sort_by_key(|line| line.timestamp);
    Ok(synchronized_lines)
}


fn retrieve_lrcmux_vocal_intervals(
    track: &TrackInformation,
) -> Result<Vec<VocalInterval>, String> {
    let result = query_lrcmux(
        &track.identity.title,
        &track.identity.artist,
        &track.identity.album,
        track.duration,
    )?;

    let synchronization_level = result.meta.level.clone();
    let mut intervals = Vec::with_capacity(result.lines.len());

    for line in result.lines {
        if line.end <= line.start || line.text.trim().is_empty() {
            continue;
        }

        intervals.push(VocalInterval {
            start: Duration::from_millis(line.start),
            end: Duration::from_millis(line.end),
            text: line.text,
        });
    }

    intervals.sort_by_key(|interval| interval.start);

    if intervals.is_empty() {
        return Err(format!(
            "LRCMUX returned no usable start/end vocal intervals for '{}' by '{}'.",
            track.identity.title, track.identity.artist
        ));
    }

    log_information(&format!(
        "[AUDIO MOTION] LRCMUX vocal timing ready: synchronization='{}' intervals={}",
        synchronization_level,
        intervals.len()
    ));

    Ok(intervals)
}

fn parse_synchronized_lyrics(lyrics: &str) -> Result<Vec<SynchronizedLine>, String> {
    let mut lines = Vec::new();

    for raw_line in lyrics.lines() {
        let line = raw_line.trim();
        if !line.starts_with('[') {
            continue;
        }

        let Some(close_bracket) = line.find(']') else {
            continue;
        };

        let timestamp_text = &line[1..close_bracket];
        let Some(timestamp) = parse_lrc_timestamp(timestamp_text) else {
            continue;
        };

        let text = line[close_bracket + 1..].trim().to_string();
        if text.is_empty() {
            continue;
        }

        lines.push(SynchronizedLine { timestamp, text });
    }

    if lines.is_empty() {
        return Err(
            "LRCLIB synchronized lyrics contained no parseable timestamped lines.".to_string(),
        );
    }

    lines.sort_by_key(|line| line.timestamp);
    Ok(lines)
}

fn parse_lrc_timestamp(timestamp: &str) -> Option<Duration> {
    let (minutes_text, seconds_text) = timestamp.split_once(':')?;
    let minutes = minutes_text.parse::<u64>().ok()?;
    let seconds = seconds_text.parse::<f64>().ok()?;

    if !seconds.is_finite() || seconds < 0.0 || seconds >= 60.0 {
        return None;
    }

    Some(Duration::from_secs_f64(minutes as f64 * 60.0 + seconds))
}

fn find_active_line(lines: &[SynchronizedLine], position: Duration) -> Option<usize> {
    if lines.is_empty() || position < lines[0].timestamp {
        return None;
    }

    match lines.binary_search_by_key(&position, |line| line.timestamp) {
        Ok(index) => Some(index),
        Err(0) => None,
        Err(index) => Some(index - 1),
    }
}

fn find_active_vocal_interval(
    intervals: &[VocalInterval],
    position: Duration,
) -> Option<usize> {
    if intervals.is_empty() || position < intervals[0].start {
        return None;
    }

    let candidate =
        match intervals.binary_search_by_key(&position, |interval| interval.start) {
            Ok(index) => index,
            Err(0) => return None,
            Err(index) => index - 1,
        };

    intervals
        .get(candidate)
        .filter(|interval| position >= interval.start && position < interval.end)
        .map(|_| candidate)
}

fn query_lrclib(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
) -> Result<LrclibResponse, String> {
    const MAX_RETRIES: usize = 3;
    const RETRY_DELAYS_SECONDS: [u64; MAX_RETRIES] = [1, 2, 4];

    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyrics"))
        .build()
        .map_err(|error| format!("Unable to create the LRCLIB HTTP client: {}", error))?;

    let mut query = vec![
        ("track_name", title.to_string()),
        ("artist_name", artist.to_string()),
        ("duration", format!("{:.3}", duration.as_secs_f64())),
    ];

    if !album.trim().is_empty() {
        query.push(("album_name", album.to_string()));
    }

    for attempt in 0..=MAX_RETRIES {
        let response = match client.get(LRCLIB_GET_URL).query(&query).send() {
            Ok(response) => response,
            Err(error) => {
                if attempt < MAX_RETRIES {
                    let delay_seconds = RETRY_DELAYS_SECONDS[attempt];
                    log_warning(&format!(
                        "[LYRICS] Unable to contact LRCLIB: {}; retrying in {} second(s) ({}/{})",
                        error,
                        delay_seconds,
                        attempt + 1,
                        MAX_RETRIES
                    ));
                    thread::sleep(Duration::from_secs(delay_seconds));
                    continue;
                }

                return Err(format!(
                    "Unable to contact LRCLIB after {} attempts: {}",
                    MAX_RETRIES + 1,
                    error
                ));
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(format!(
                "LRCLIB did not find a matching track for '{}' by '{}'.",
                title, artist
            ));
        }

        let retryable_status =
            status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

        if retryable_status && attempt < MAX_RETRIES {
            let retry_after_seconds = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse::<u64>().ok());
            let delay_seconds =
                retry_after_seconds.unwrap_or(RETRY_DELAYS_SECONDS[attempt]);

            log_warning(&format!(
                "[LYRICS] LRCLIB returned HTTP status {}; retrying in {} second(s) ({}/{})",
                status,
                delay_seconds,
                attempt + 1,
                MAX_RETRIES
            ));
            thread::sleep(Duration::from_secs(delay_seconds));
            continue;
        }

        if !status.is_success() {
            return Err(format!("LRCLIB returned HTTP status {}.", status));
        }

        return response
            .json::<LrclibResponse>()
            .map_err(|error| format!("Unable to decode the LRCLIB response: {}", error));
    }

    Err("LRCLIB retry loop ended unexpectedly.".to_string())
}

fn query_lrcmux(
    title: &str,
    artist: &str,
    album: &str,
    duration: Duration,
) -> Result<LrcmuxResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyrics"))
        .build()
        .map_err(|error| format!("Unable to create the LRCMUX HTTP client: {}", error))?;

    let mut query = vec![
        ("artist", artist.to_string()),
        ("title", title.to_string()),
        ("duration", duration.as_secs().to_string()),
    ];

    if !album.trim().is_empty() {
        query.push(("album", album.to_string()));
    }

    let response = client
        .get(LRCMUX_GET_URL)
        .query(&query)
        .send()
        .map_err(|error| format!("Unable to contact LRCMUX: {}", error))?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(format!(
            "LRCMUX did not find a matching track for '{}' by '{}'.",
            title, artist
        ));
    }

    if !status.is_success() {
        return Err(format!("LRCMUX returned HTTP status {}.", status));
    }

    response
        .json::<LrcmuxResponse>()
        .map_err(|error| format!("Unable to decode the LRCMUX response: {}", error))
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn log_information(message: &str) {
    crate::logger::information(&crate::locate_paths::runtime_log_path(), message);
}

fn log_warning(message: &str) {
    crate::logger::warning(&crate::locate_paths::runtime_log_path(), message);
}
