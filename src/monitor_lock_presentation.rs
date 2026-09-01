use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Rolling observation window for lock-presentation measurements.
///
/// This intentionally matches the five-second horizon used by the existing
/// renderer FPS monitor, but it is a separate data set with separate semantics.
/// No WARNING or CRITICAL policy is applied by this module yet.
const OBSERVATION_WINDOW: Duration = Duration::from_secs(5);

/// Emit one informational observation after each completed observation window.
const REPORT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockPresentationBackend {
    Gnome,
    Wayland,
    Kde,
}

impl LockPresentationBackend {
    fn name(self) -> &'static str {
        match self {
            Self::Gnome => "GNOME",
            Self::Wayland => "Wayland",
            Self::Kde => "KDE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LockPresentationHealth {
    #[default]
    Normal,
    Warning,
    Critical,
}

/// One completed backend-presentation sample.
///
/// Milestone 2 keeps GNOME's two preparation costs separate so GPU readback and
/// CPU row reversal can be observed independently. Other backends may leave a
/// component at `Duration::ZERO` when that stage does not exist in their path.
///
/// `total` spans all Screenshaver-owned presentation work after
/// FrameRenderEngine returns and before normal frame pacing begins.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LockPresentationSample {
    pub configured_fps: u32,
    pub readback: Duration,
    pub row_flip: Duration,
    pub transfer: Duration,
    pub submit: Duration,
    pub total: Duration,
}

#[derive(Debug, Clone, Copy)]
struct TimedSample {
    timestamp: Instant,
    sample: LockPresentationSample,
}

/// Shared lock-presentation health/measurement service.
///
/// Milestone 1 is intentionally observation-only. The monitor gathers a rolling
/// set of lock-presentation samples and periodically writes measurements to the
/// runtime log. It does not currently classify samples as WARNING/CRITICAL and
/// therefore cannot request a fallback. That policy will be added only after
/// measurements from real lock sessions have established appropriate limits.
pub(crate) struct LockPresentationMonitor {
    backend: LockPresentationBackend,
    logfile: PathBuf,
    samples: VecDeque<TimedSample>,
    configured_fps: u32,
    last_report: Instant,
    health: LockPresentationHealth,
}

impl LockPresentationMonitor {
    pub(crate) fn new(
        backend: LockPresentationBackend,
        logfile: &Path,
        configured_fps: u32,
    ) -> Self {
        let configured_fps = configured_fps.max(1);

        crate::logger::information(
            logfile,
            &format!(
                "[LOCK] {} lock-presentation monitor enabled in observation-only mode; configured_fps={}",
                backend.name(),
                configured_fps,
            ),
        );

        Self {
            backend,
            logfile: logfile.to_path_buf(),
            samples: VecDeque::new(),
            configured_fps,
            last_report: Instant::now(),
            health: LockPresentationHealth::Normal,
        }
    }

    pub(crate) fn health(&self) -> LockPresentationHealth {
        self.health
    }

    pub(crate) fn record(&mut self, sample: LockPresentationSample) {
        let now = Instant::now();
        let configured_fps = sample.configured_fps.max(1);

        if configured_fps != self.configured_fps {
            crate::logger::information(
                &self.logfile,
                &format!(
                    "[LOCK] {} lock-presentation observation target changed: configured_fps={} -> {}; resetting measurement window",
                    self.backend.name(),
                    self.configured_fps,
                    configured_fps,
                ),
            );

            self.samples.clear();
            self.configured_fps = configured_fps;
            self.last_report = now;
        }

        self.samples.push_back(TimedSample {
            timestamp: now,
            sample: LockPresentationSample {
                configured_fps,
                ..sample
            },
        });

        self.prune(now);

        if now.duration_since(self.last_report) >= REPORT_INTERVAL {
            self.log_observation();
            self.last_report = now;
        }
    }

    fn prune(&mut self, now: Instant) {
        while let Some(front) = self.samples.front() {
            if now.duration_since(front.timestamp) <= OBSERVATION_WINDOW {
                break;
            }

            self.samples.pop_front();
        }
    }

    fn log_observation(&self) {
        if self.samples.is_empty() {
            return;
        }

        let count = self.samples.len() as u32;
        let count_f64 = count as f64;

        let mut readback_total = Duration::ZERO;
        let mut row_flip_total = Duration::ZERO;
        let mut transfer_total = Duration::ZERO;
        let mut submit_total = Duration::ZERO;
        let mut presentation_total = Duration::ZERO;
        let mut maximum_total = Duration::ZERO;

        for timed in &self.samples {
            readback_total += timed.sample.readback;
            row_flip_total += timed.sample.row_flip;
            transfer_total += timed.sample.transfer;
            submit_total += timed.sample.submit;
            presentation_total += timed.sample.total;
            maximum_total = maximum_total.max(timed.sample.total);
        }

        let average_readback_ms = readback_total.as_secs_f64() * 1000.0 / count_f64;
        let average_row_flip_ms = row_flip_total.as_secs_f64() * 1000.0 / count_f64;
        let average_transfer_ms = transfer_total.as_secs_f64() * 1000.0 / count_f64;
        let average_submit_ms = submit_total.as_secs_f64() * 1000.0 / count_f64;
        let average_total_ms = presentation_total.as_secs_f64() * 1000.0 / count_f64;
        let maximum_total_ms = maximum_total.as_secs_f64() * 1000.0;

        let frame_budget_ms = 1000.0 / self.configured_fps.max(1) as f64;
        let average_budget_usage = if frame_budget_ms > 0.0 {
            average_total_ms / frame_budget_ms * 100.0
        } else {
            0.0
        };

        crate::logger::information(
            &self.logfile,
            &format!(
                "[LOCK] {} lock-presentation observation: samples={} configured_fps={} frame_budget_ms={:.3} avg_readback_ms={:.3} avg_row_flip_ms={:.3} avg_transfer_ms={:.3} avg_submit_ms={:.3} avg_total_ms={:.3} max_total_ms={:.3} avg_budget_usage={:.1}% health=OBSERVATION_ONLY",
                self.backend.name(),
                count,
                self.configured_fps,
                frame_budget_ms,
                average_readback_ms,
                average_row_flip_ms,
                average_transfer_ms,
                average_submit_ms,
                average_total_ms,
                maximum_total_ms,
                average_budget_usage,
            ),
        );
    }
}
