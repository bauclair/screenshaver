use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock, mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const LRCLIB_GET_URL: &str = "https://lrclib.net/api/get";
const LRCMUX_GET_URL: &str = "https://api.lrcmux.dev/get";
const POSITION_POLL_INTERVAL: Duration = Duration::from_millis(50);
const METADATA_POLL_INTERVAL: Duration = Duration::from_millis(500);
const PLAYER_SCAN_INTERVAL: Duration = Duration::from_millis(500);
const FALLBACK_CLOCK_MIN_TRUSTED_POSITION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackProgress {
    pub position: Duration,
    pub duration: Duration,
    pub observed_at: Instant,
    pub playing: bool,
}

impl TrackProgress {
    pub fn fraction(&self) -> f64 {
        if self.duration.is_zero() { return 0.0; }
        (self.position.saturating_add(if self.playing { self.observed_at.elapsed() } else { Duration::ZERO })
            .as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LyricsState {
    pub previous: Option<String>,
    pub current: Option<String>,
    pub next: Option<String>,
    pub secondary_current: Option<String>,
    pub progress: Option<TrackProgress>,
    pub seekable: bool,
    pub can_go_previous: bool,
    pub can_go_next: bool,
}

impl LyricsState {
    pub fn same_text(&self, other: &Self) -> bool {
        self.previous == other.previous && self.current == other.current
            && self.next == other.next && self.secondary_current == other.secondary_current
    }
}

pub type SharedLyricsState = Arc<Mutex<LyricsState>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocalTimingState {
    pub available: bool,
    pub active: bool,
    pub current_line: Option<String>,
}

pub type SharedVocalTimingState = Arc<Mutex<VocalTimingState>>;

pub fn shared_audio_motion_vocal_timing_state() -> SharedVocalTimingState {
    static SHARED: OnceLock<SharedVocalTimingState> = OnceLock::new();
    Arc::clone(SHARED.get_or_init(|| Arc::new(Mutex::new(VocalTimingState::default()))))
}

/// A seek request is interpreted by the MPRIS worker, never by the renderer.
/// `fraction` is normalized to the current track duration.
#[derive(Debug, Clone, Copy)]
struct SeekRequest {
    fraction: f64,
}

#[derive(Debug, Clone)]
pub struct LyricsSeekHandle {
    sender: mpsc::Sender<SeekRequest>,
    state: SharedLyricsState,
}

impl LyricsSeekHandle {
    /// Returns false if the manager is stopped or no usable duration is known.
    pub fn request_fraction(&self, fraction: f64) -> bool {
        if !fraction.is_finite() { return false; }
        let available = self.state.lock().ok()
            .and_then(|snapshot| snapshot.progress.as_ref().map(|p| snapshot.seekable && !p.duration.is_zero()))
            .unwrap_or(false);
        available && self.sender.send(SeekRequest { fraction: fraction.clamp(0.0, 1.0) }).is_ok()
    }
}

/// Navigation requests are handled on the existing MPRIS worker thread.
#[derive(Debug, Clone, Copy)]
enum NavigationRequest { Previous, Next }

#[derive(Debug, Clone)]
pub struct LyricsNavigationHandle {
    sender: mpsc::Sender<NavigationRequest>,
    state: SharedLyricsState,
}

impl LyricsNavigationHandle {
    pub fn previous(&self) -> bool { self.request(NavigationRequest::Previous) }
    pub fn next(&self) -> bool { self.request(NavigationRequest::Next) }

    fn request(&self, command: NavigationRequest) -> bool {
        let allowed = self.state.lock().ok().is_some_and(|snapshot| match command {
            NavigationRequest::Previous => snapshot.can_go_previous,
            NavigationRequest::Next => snapshot.can_go_next,
        });
        allowed && self.sender.send(command).is_ok()
    }
}

pub struct LyricsManager {
    state: SharedLyricsState,
    vocal_timing_state: SharedVocalTimingState,
    seek_sender: mpsc::Sender<SeekRequest>,
    navigation_sender: mpsc::Sender<NavigationRequest>,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LyricsManager {
    pub fn start() -> Result<Self, String> {
        Self::start_with_translation(None)
    }

    pub fn start_with_translation(target_locale: Option<String>) -> Result<Self, String> {
        Self::start_with_translation_options(target_locale, false)
    }

    pub fn start_with_translation_options(target_locale: Option<String>, show_original_lyrics: bool) -> Result<Self, String> {
        // Verify that the session D-Bus is reachable before reporting a
        // successfully started manager. The worker owns all later MPRIS access.
        PlayerFinder::new().map_err(|error| {
            format!(
                "Unable to connect to the session D-Bus for MPRIS discovery: {}",
                error
            )
        })?;

        let state = Arc::new(Mutex::new(LyricsState::default()));
        let vocal_timing_state = shared_audio_motion_vocal_timing_state();
        let running = Arc::new(AtomicBool::new(true));
        let (seek_sender, seek_receiver) = mpsc::channel::<SeekRequest>();
        let (navigation_sender, navigation_receiver) = mpsc::channel::<NavigationRequest>();
        let worker_state = Arc::clone(&state);
        let worker_vocal_timing_state = Arc::clone(&vocal_timing_state);
        let worker_running = Arc::clone(&running);

        let worker = thread::Builder::new()
            .name("screenshaver-lyrics".to_string())
            .spawn(move || {
                run_worker(worker_state, worker_vocal_timing_state, worker_running, seek_receiver, navigation_receiver, target_locale, show_original_lyrics);
            })
            .map_err(|error| format!("Unable to start lyrics worker: {}", error))?;

        log_information("[LYRICS] Production lyrics manager started");

        Ok(Self {
            state,
            vocal_timing_state,
            seek_sender,
            navigation_sender,
            running,
            worker: Some(worker),
        })
    }

    pub fn seek_handle(&self) -> LyricsSeekHandle {
        LyricsSeekHandle { sender: self.seek_sender.clone(), state: Arc::clone(&self.state) }
    }

    pub fn navigation_handle(&self) -> LyricsNavigationHandle {
        LyricsNavigationHandle { sender: self.navigation_sender.clone(), state: Arc::clone(&self.state) }
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

// Network lookups must never run on the MPRIS polling/seek thread.
// A single retrieval worker prevents unbounded requests when tracks change rapidly.
struct RetrievedTrack {
    generation: u64,
    identity: TrackIdentity,
    lines: Vec<SynchronizedLine>,
    intervals: Vec<VocalInterval>,
}

fn start_track_retrieval(
    track: TrackInformation,
    generation: u64,
    sender: mpsc::Sender<RetrievedTrack>,
) -> Result<JoinHandle<()>, String> {
    thread::Builder::new()
        .name("screenshaver-lyric-retrieval".into())
        .spawn(move || {
            let lines = retrieve_synchronized_lines(&track).unwrap_or_else(|error| {
                log_warning(&format!("[LYRICS] {}", error));
                Vec::new()
            });
            let intervals = retrieve_lrcmux_vocal_intervals(&track).unwrap_or_else(|error| {
                log_warning(&format!("[AUDIO MOTION] LRCMUX vocal timing unavailable: {}", error));
                Vec::new()
            });
            let _ = sender.send(RetrievedTrack {
                generation, identity: track.identity, lines, intervals,
            });
        })
        .map_err(|error| format!("Unable to start lyric retrieval: {}", error))
}

fn run_worker(
    state: Arc<Mutex<LyricsState>>,
    vocal_timing_state: Arc<Mutex<VocalTimingState>>,
    running: Arc<AtomicBool>,
    seek_receiver: mpsc::Receiver<SeekRequest>,
    navigation_receiver: mpsc::Receiver<NavigationRequest>,
    target_locale: Option<String>,
    show_original_lyrics: bool,
) {
    let target_language = target_locale.as_deref().and_then(|v| v.split(['-', '_']).next()).map(str::to_ascii_lowercase).filter(|v| !v.is_empty() && v != "en");
    let (translation_tx, translation_rx) = mpsc::channel::<(u64, Result<Vec<(usize, String)>, String>)>();
    let mut translation_generation = 0u64;
    let mut translation_cancel: Option<Arc<AtomicBool>> = None;
    let mut translated_lines: Option<Vec<SynchronizedLine>> = None;
    let mut translation_started_for_track = false;
    let mut translation_workers: Vec<JoinHandle<()>> = Vec::new();
    let mut last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
    let mut last_metadata_poll = Instant::now() - METADATA_POLL_INTERVAL;
    let mut last_player_scan = Instant::now() - PLAYER_SCAN_INTERVAL;
    let mut active_player: Option<Player> = None;
    let mut current_track: Option<TrackInformation> = None;
    let mut synchronized_lines: Vec<SynchronizedLine> = Vec::new();
    let mut vocal_intervals: Vec<VocalInterval> = Vec::new();
    let mut last_line_index: Option<usize> = None;
    let mut last_vocal_interval_index: Option<usize> = None;
    // Firefox/regular YouTube can keep reporting Playing while MPRIS Position
    // collapses to zero and mpris:length disappears.  Keep the last trustworthy
    // nonzero position and its monotonic observation time so lyrics can continue
    // across that specific broken-MPRIS state.
    let mut last_trusted_position: Option<(Duration, Instant)> = None;
    let mut fallback_clock_active = false;
    let mut last_seekability_poll = Instant::now() - METADATA_POLL_INTERVAL;
    let mut player_can_seek = false;
    let (retrieval_tx, retrieval_rx) = mpsc::channel::<RetrievedTrack>();
    let mut retrieval_generation = 0u64;
    let mut retrieval_worker: Option<JoinHandle<()>> = None;
    let mut queued_retrieval: Option<(TrackInformation, u64)> = None;


    while running.load(Ordering::SeqCst) {
        // Process navigation independently of lyrics downloads and translation.
        // Coalesce queued navigation clicks into one command per iteration.
        let mut pending_navigation = None;
        while let Ok(command) = navigation_receiver.try_recv() { pending_navigation = Some(command); }
        if let (Some(command), Some(player)) = (pending_navigation, active_player.as_ref()) {
            let result = match command {
                NavigationRequest::Previous if player.can_go_previous().unwrap_or(false) => Some(player.previous()),
                NavigationRequest::Next if player.can_go_next().unwrap_or(false) => Some(player.next()),
                _ => None,
            };
            if let Some(result) = result {
                if let Err(error) = result {
                    log_warning(&format!("[LYRICS] MPRIS navigation rejected: {}", error));
                } else {
                    last_metadata_poll = Instant::now() - METADATA_POLL_INTERVAL;
                    last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
                }
            }
        }

        // Keep only the newest click if several arrive before a worker iteration.
        let mut pending_seek = None;
        while let Ok(request) = seek_receiver.try_recv() { pending_seek = Some(request); }
        if let Some(request) = pending_seek {
            if let (Some(player), Some(track)) = (active_player.as_ref(), current_track.as_ref()) {
                if !track.duration.is_zero() && player.can_seek().unwrap_or(false) {
                    // Resolve the track ID at execution time. This prevents a stale
                    // click from targeting a previously selected MPRIS track.
                    match player.get_metadata() {
                        Ok(metadata) => {
                            if let Some(track_id) = metadata.track_id() {
                                // Reject a queued request if the song changed since the
                                // last metadata refresh.
                                let same_track = metadata.title() == Some(track.identity.title.as_str())
                                    && metadata.artists().and_then(|a| a.first().copied())
                                        == Some(track.identity.artist.as_str());
                                if !same_track { continue; }
                                let requested = Duration::from_secs_f64(
                                    track.duration.as_secs_f64() * request.fraction
                                );
                                if let Err(error) = player.set_position(track_id, &requested) {
                                    log_warning(&format!("[LYRICS] MPRIS seek rejected: {}", error));
                                } else {
                                    // Do not claim success in the displayed clock; the
                                    // next position poll reconciles actual player state.
                                    last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
                        last_seekability_poll = Instant::now() - METADATA_POLL_INTERVAL;
                                    last_trusted_position = None;
                                    fallback_clock_active = false;
                                }
                            }
                        }
                        Err(error) => log_warning(&format!("[LYRICS] Cannot inspect track for seek: {}", error)),
                    }
                }
            }
        }

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
                    retrieval_generation = retrieval_generation.wrapping_add(1);
                    queued_retrieval = None;
                    translation_generation = translation_generation.wrapping_add(1);
                    if let Some(cancel) = translation_cancel.take() { cancel.store(true, Ordering::Release); }
                    translated_lines = None;
                    translation_started_for_track = false;
                    synchronized_lines.clear();
                    vocal_intervals.clear();
                    last_line_index = None;
                    last_vocal_interval_index = None;
                    last_trusted_position = None;
                    fallback_clock_active = false;
                    if let Ok(mut snapshot) = state.lock() {
                        if status == PlaybackStatus::Paused {
                            if let Some(progress) = snapshot.progress.as_mut() {
                                progress.position = progress.position.saturating_add(progress.observed_at.elapsed());
                                progress.observed_at = Instant::now();
                                progress.playing = false;
                            }
                        } else { snapshot.progress = None; }
                    }
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
                    retrieval_generation = retrieval_generation.wrapping_add(1);
                    queued_retrieval = None;
                    translation_generation = translation_generation.wrapping_add(1);
                    if let Some(cancel) = translation_cancel.take() { cancel.store(true, Ordering::Release); }
                    translated_lines = None;
                    translation_started_for_track = false;
                    synchronized_lines.clear();
                    vocal_intervals.clear();
                    last_line_index = None;
                    last_vocal_interval_index = None;
                    last_trusted_position = None;
                    fallback_clock_active = false;
                    if let Ok(mut snapshot) = state.lock() { snapshot.progress = None; }
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
                    Ok(track) => {
                        log_information(&format!(
                            "[LYRICS] Following MPRIS player '{}' for '{}' by '{}'",
                            player.identity(), track.identity.title, track.identity.artist
                        ));
                        // A newly selected player starts a distinct translation generation.
                        translation_generation = translation_generation.wrapping_add(1);
                        if let Some(cancel) = translation_cancel.take() {
                            cancel.store(true, Ordering::Release);
                        }
                        translated_lines = None;
                        translation_started_for_track = false;
                        retrieval_generation = retrieval_generation.wrapping_add(1);
                        queued_retrieval = Some((track.clone(), retrieval_generation));
                        current_track = Some(track);
                        synchronized_lines.clear();
                        vocal_intervals.clear();
                        last_line_index = None;
                        last_vocal_interval_index = None;
                        active_player = Some(player);
                        last_trusted_position = None;
                        fallback_clock_active = false;
                        last_metadata_poll = Instant::now();
                        last_position_poll = Instant::now() - POSITION_POLL_INTERVAL;
                        if let Ok(mut snapshot) = state.lock() { snapshot.progress = None; snapshot.seekable = false; }
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
                match read_track_information(player, Some(track)) {
                    Ok(observed_track) => {
                        if observed_track.identity != track.identity {
                            log_information(&format!(
                                "[LYRICS] Track change detected on '{}': '{}' by '{}'",
                                player.identity(),
                                observed_track.identity.title,
                                observed_track.identity.artist
                            ));

                            // Invalidate any in-flight translation before loading the next song.
                            translation_generation = translation_generation.wrapping_add(1);
                            if let Some(cancel) = translation_cancel.take() {
                                cancel.store(true, Ordering::Release);
                            }
                            translated_lines = None;
                            translation_started_for_track = false;
                            *track = observed_track;
                            retrieval_generation = retrieval_generation.wrapping_add(1);
                            queued_retrieval = Some((track.clone(), retrieval_generation));
                            synchronized_lines.clear();
                            vocal_intervals.clear();
                            last_line_index = None;
                            last_vocal_interval_index = None;
                            last_trusted_position = None;
                            fallback_clock_active = false;
                            last_seekability_poll = Instant::now() - METADATA_POLL_INTERVAL;
                            if let Ok(mut snapshot) = state.lock() { snapshot.progress = None; snapshot.seekable = false; }
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

        // Install only the newest matching retrieval result. Old network requests
        // may finish after a track change and must never replace current lyrics.
        while let Ok(result) = retrieval_rx.try_recv() {
            if result.generation != retrieval_generation
                || current_track.as_ref().is_none_or(|track| track.identity != result.identity)
            { continue; }
            synchronized_lines = result.lines;
            vocal_intervals = result.intervals;
            translated_lines = None;
            translation_started_for_track = false;
            last_line_index = None;
            last_vocal_interval_index = None;
            publish_vocal_timing_state(&vocal_timing_state, VocalTimingState {
                available: !vocal_intervals.is_empty(), active: false, current_line: None,
            });
            if let Some((position, at)) = last_trusted_position {
                let effective = if fallback_clock_active { position.saturating_add(at.elapsed()) } else { position };
                let active = find_active_line(&synchronized_lines, effective);
                publish_display_line_state(&state, &synchronized_lines, None, active, show_original_lyrics);
                last_line_index = active;
                let vocal = find_active_vocal_interval(&vocal_intervals, effective);
                publish_vocal_interval_state(&vocal_timing_state, &vocal_intervals, vocal);
                last_vocal_interval_index = vocal;
            }
        }
        // Never wait for a previous HTTP request: queue the newest track and
        // launch it when the single retrieval slot becomes available.
        if retrieval_worker.as_ref().is_some_and(|worker| worker.is_finished()) {
            if let Some(worker) = retrieval_worker.take() { let _ = worker.join(); }
        }
        if retrieval_worker.is_none() {
            if let Some((track, generation)) = queued_retrieval.take() {
                match start_track_retrieval(track.clone(), generation, retrieval_tx.clone()) {
                    Ok(worker) => retrieval_worker = Some(worker),
                    Err(error) => log_warning(&format!("[LYRICS] {}", error)),
                }
            }
        }

        // Translation never blocks position polling. Results are accepted only for
        // the active generation; all timestamps remain those of the source lines.
        while let Ok((generation, result)) = translation_rx.try_recv() {
            if generation != translation_generation || current_track.is_none() { continue; }
            match result {
                Ok(batch) => {
                    // A completed batch is usable immediately; all other lines
                    // retain their originals until their own translation arrives.
                    let translated = translated_lines.get_or_insert_with(|| synchronized_lines.clone());
                    for (index, text) in batch {
                        if let Some(line) = translated.get_mut(index) {
                            line.text = text;
                        }
                    }
                    // Rebuild the visible line even if the active index has not
                    // changed. This is what makes partial results appear promptly.
                    if let Some((position, observed_at)) = last_trusted_position {
                        let effective_position = if fallback_clock_active {
                            position.saturating_add(observed_at.elapsed())
                        } else { position };
                        let active = find_active_line(&synchronized_lines, effective_position);
                        publish_display_line_state(&state, &synchronized_lines,
                            translated_lines.as_deref(), active, show_original_lyrics);
                        last_line_index = active;
                    }
                }
                Err(error) => log_warning(&format!("[LYRICS] Translation batch unavailable; originals retained: {}", error)),
            }
        }

        if last_seekability_poll.elapsed() >= METADATA_POLL_INTERVAL {
            last_seekability_poll = Instant::now();
            player_can_seek = active_player.as_ref()
                .and_then(|player| player.can_seek().ok()).unwrap_or(false);
            let can_go_previous = active_player.as_ref()
                .and_then(|player| player.can_go_previous().ok()).unwrap_or(false);
            let can_go_next = active_player.as_ref()
                .and_then(|player| player.can_go_next().ok()).unwrap_or(false);
            if let Ok(mut snapshot) = state.lock() {
                snapshot.can_go_previous = can_go_previous;
                snapshot.can_go_next = can_go_next;
            }
        }

        if last_position_poll.elapsed() >= POSITION_POLL_INTERVAL {
            last_position_poll = Instant::now();

            if let Some(player) = active_player.as_ref() {
                let position_result = player.get_position();

                if let Ok(reported_position) = position_result {
                    let now = Instant::now();

                    // A normal nonzero MPRIS position is always authoritative.
                    // It also ends any temporary fallback immediately.
                    let effective_position = if reported_position > Duration::ZERO {
                        if fallback_clock_active {
                            log_information(&format!(
                                "[LYRICS] MPRIS position recovered on '{}'; returning to the player clock at {:.3}s",
                                player.identity(),
                                reported_position.as_secs_f64()
                            ));
                        }
                        fallback_clock_active = false;
                        last_trusted_position = Some((reported_position, now));
                        reported_position
                    } else {
                        let duration_missing = player
                            .get_metadata()
                            .ok()
                            .and_then(|metadata| metadata.length())
                            .is_none();

                        let fallback_candidate = last_trusted_position
                            .filter(|(position, _)| {
                                *position >= FALLBACK_CLOCK_MIN_TRUSTED_POSITION
                            });

                        if duration_missing {
                            if let Some((trusted_position, trusted_at)) = fallback_candidate {
                                if !fallback_clock_active {
                                    log_warning(&format!(
                                        "[LYRICS] MPRIS position collapsed to zero while '{}' remains Playing and duration is missing; continuing with guarded fallback clock from {:.3}s",
                                        player.identity(),
                                        trusted_position.as_secs_f64()
                                    ));
                                }
                                fallback_clock_active = true;
                                trusted_position.saturating_add(trusted_at.elapsed())
                            } else {
                                reported_position
                            }
                        } else {
                            // Duration is still present, so zero is treated as a real seek/start
                            // rather than the Firefox failure signature.
                            fallback_clock_active = false;
                            last_trusted_position = Some((reported_position, now));
                            reported_position
                        }
                    };

                    if let Some(track) = current_track.as_ref() {
                        if let Ok(mut snapshot) = state.lock() {
                            snapshot.seekable = track.duration > Duration::ZERO
                                && player_can_seek
                                && !fallback_clock_active;
                            snapshot.progress = (track.duration > Duration::ZERO).then_some(TrackProgress {
                                position: effective_position,
                                duration: track.duration,
                                observed_at: now,
                                playing: true,
                            });
                        }
                    }

                    let active = find_active_line(&synchronized_lines, effective_position);
                    if active != last_line_index {
                        last_line_index = active;
                        publish_display_line_state(
                            &state, &synchronized_lines, translated_lines.as_deref(),
                            active, show_original_lyrics,
                        );
                    }

                    let vocal_active =
                        find_active_vocal_interval(&vocal_intervals, effective_position);
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

        translation_workers.retain(|handle| !handle.is_finished());
        if !translation_started_for_track && !synchronized_lines.is_empty() {
            if let Some(target) = target_language.as_deref() {
                // Avoid unbounded detached requests when songs are skipped rapidly.
                if translation_workers.len() < 2 {
                    translation_started_for_track = true;
                    let source: Vec<String> = synchronized_lines.iter().map(|l| l.text.clone()).collect();
                    let cancelled = Arc::new(AtomicBool::new(false));
                    translation_cancel = Some(Arc::clone(&cancelled));
                    let sender = translation_tx.clone();
                    let generation = translation_generation;
                    let target = target.to_owned();
                    // Prioritize the current lyric and the next few lines, not
                    // necessarily the beginning of the track after a seek.
                    let start_index = last_trusted_position
                        .and_then(|(position, at)| {
                            let effective = if fallback_clock_active {
                                position.saturating_add(at.elapsed())
                            } else { position };
                            find_active_line(&synchronized_lines, effective)
                        }).unwrap_or(0);
                    match thread::Builder::new().name("screenshaver-lyric-translation".into()).spawn(move || {
                        translate_production_lines_incrementally(
                            &source, &target, &cancelled, start_index, |batch| {
                                let _ = sender.send((generation, batch));
                            });
                    }) {
                        Ok(handle) => translation_workers.push(handle),
                        Err(error) => log_warning(&format!("[LYRICS] Unable to start translation: {}", error)),
                    }
                }
            }
        }


        thread::sleep(Duration::from_millis(10));
    }

    if let Some(cancel) = translation_cancel { cancel.store(true, Ordering::Release); }
    if let Ok(mut snapshot) = state.lock() { snapshot.progress = None; }
                    publish_state(&state, LyricsState::default());
    publish_vocal_timing_state(&vocal_timing_state, VocalTimingState::default());
}

fn publish_display_line_state(
    state: &Arc<Mutex<LyricsState>>,
    originals: &[SynchronizedLine],
    translations: Option<&[SynchronizedLine]>,
    active: Option<usize>,
    show_original_lyrics: bool,
) {
    let primary = translations.unwrap_or(originals);
    let secondary = if show_original_lyrics && translations.is_some() {
        active.and_then(|index| {
            let original = originals.get(index)?.text.trim();
            let translated = primary.get(index)?.text.trim();
            if original.is_empty() || original == translated { None }
            else { Some(original.to_string()) }
        })
    } else { None };
    let Some(active) = active else {
        publish_state(state, LyricsState::default());
        return;
    };
    publish_state(state, LyricsState {
        previous: active.checked_sub(1).and_then(|i| primary.get(i)).map(|l| l.text.clone()),
        current: primary.get(active).map(|l| l.text.clone()),
        next: primary.get(active + 1).map(|l| l.text.clone()),
        secondary_current: secondary,
        progress: None,
        seekable: false,
        can_go_previous: false,
        can_go_next: false,
    });
}

fn publish_state(state: &Arc<Mutex<LyricsState>>, next_state: LyricsState) {
    if let Ok(mut state) = state.lock() {
        let mut next_state = next_state;
        next_state.progress = state.progress.take();
        next_state.seekable = state.seekable;
        next_state.can_go_previous = state.can_go_previous;
        next_state.can_go_next = state.can_go_next;
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

fn load_player_track(player: &Player) -> Result<TrackInformation, String> {
    read_track_information(player, None)
}

fn read_track_information(
    player: &Player,
    previous_track: Option<&TrackInformation>,
) -> Result<TrackInformation, String> {
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
    let duration = match metadata.length() {
        Some(duration) => duration,
        None => previous_track
            .map(|track| track.duration)
            .ok_or_else(|| {
                "The selected MPRIS player did not provide a track duration.".to_string()
            })?,
    };

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


// No persistent translation cache. Translate the active lyric first, followed
// by the next four, then the remaining lines in playback order. Publish each
// successful request independently; a failed request never discards earlier work.
fn translate_production_lines_incrementally(
    source: &[String], target: &str, cancelled: &AtomicBool,
    start_index: usize, mut publish: impl FnMut(Result<Vec<(usize, String)>, String>),
) {
    const MAX_CHUNK_BYTES: usize = 380;
    const PRIORITY_LINES: usize = 5;
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("Screenshaver/", env!("CARGO_PKG_VERSION"), " lyric-translation"))
        .build() {
            Ok(client) => client,
            Err(error) => { publish(Err(error.to_string())); return; }
        };
    let protected: Vec<ProtectedLine> = source.iter().map(|line| protect_vocalizations(line)).collect();
    let start = start_index.min(source.len());
    let mut order: Vec<usize> = (start..source.len()).chain(0..start).collect();
    // The first request is intentionally small to reduce time to first display.
    // Subsequent requests can use the full byte budget.
    let mut priority_remaining = PRIORITY_LINES;
    while !order.is_empty() {
        if cancelled.load(Ordering::Acquire) { return; }
        let limit = if priority_remaining > 0 { priority_remaining } else { usize::MAX };
        let mut batch = Vec::new();
        let mut bytes = 0usize;
        while !order.is_empty() && batch.len() < limit {
            let index = order[0];
            let item = &protected[index];
            if item.core.trim().is_empty() {
                order.remove(0);
                continue;
            }
            let next = item.core.len() + usize::from(!batch.is_empty());
            if next > MAX_CHUNK_BYTES {
                publish(Err(format!("Lyric {} exceeds translation request limit", index)));
                order.remove(0);
                continue;
            }
            if bytes + next > MAX_CHUNK_BYTES { break; }
            bytes += next;
            batch.push(index);
            order.remove(0);
        }
        if batch.is_empty() { continue; }
        priority_remaining = priority_remaining.saturating_sub(batch.len());
        let input = batch.iter().map(|&i| protected[i].core.as_str()).collect::<Vec<_>>().join("\n");
        match translate_chunk(&client, &input, "en", target, batch.len()) {
            Ok((response, _provider)) => {
                if cancelled.load(Ordering::Acquire) { return; }
                let lines: Vec<&str> = response.split('\n').collect();
                if lines.len() != batch.len() || lines.iter().any(|line| line.trim().is_empty()) {
                    publish(Err("Translation alignment mismatch".into()));
                    continue;
                }
                let updates = batch.iter().zip(lines).map(|(&i, text)|
                    (i, protected[i].restore(text.trim()))).collect();
                publish(Ok(updates));
            }
            Err(error) => {
                if cancelled.load(Ordering::Acquire) { return; }
                publish(Err(error));
            }
        }
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

