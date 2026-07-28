use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Length of the rolling frame-time window used to evaluate renderer
/// performance.
pub const FPS_AVERAGE_WINDOW: Duration = Duration::from_secs(5);

/// Blink interval used by renderers that visually flash a critical FPS
/// warning. Renderers that do not display an overlay may ignore this value.
pub const FPS_CRITICAL_BLINK_INTERVAL: Duration = Duration::from_millis(500);

/// Performance state derived from the rolling average frame time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FpsWarningState {
    /// Rendering is within the acceptable range for the configured FPS.
    #[default]
    Normal,

    /// Average frame time is more than 1.5 times the ideal frame time.
    Warning,

    /// Average frame time is more than 2.0 times the ideal frame time.
    Critical,

    /// Presentation-only state used when hiding the FPS text during a
    /// critical-warning blink cycle.
    ///
    /// `FrameTimeWindow` never returns this state directly.
    CriticalHidden,
}

/// Result returned after recording a rendered frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsPerformanceStatus {
    /// Approximate FPS derived from the current rolling average frame time.
    pub average_fps: u32,

    /// Average render duration of the samples currently retained in the
    /// rolling window.
    pub average_frame_time: Duration,

    /// Warning state calculated relative to the configured FPS target.
    pub warning_state: FpsWarningState,

    /// Number of frame-time samples currently represented by the rolling
    /// average.
    pub sample_count: usize,
}

impl Default for FpsPerformanceStatus {
    fn default() -> Self {
        Self {
            average_fps: 0,
            average_frame_time: Duration::ZERO,
            warning_state: FpsWarningState::Normal,
            sample_count: 0,
        }
    }
}

/// Maintains a rolling window of frame-render durations and converts them into
/// a shared FPS performance state for both screensaver and wallpaper renderers.
#[derive(Debug, Default)]
pub struct FrameTimeWindow {
    samples: VecDeque<(Instant, Duration)>,
    total: Duration,
}

impl FrameTimeWindow {
    /// Creates an empty rolling frame-time window.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all retained samples and resets the rolling average.
    ///
    /// This should be called whenever the active shader changes so the new
    /// shader is evaluated independently of the previous shader.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.total = Duration::ZERO;
    }

    /// Returns `true` when the monitor currently has no retained samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns the number of frame-time samples currently retained.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Records one frame-render duration and returns the updated performance
    /// status.
    ///
    /// The configured FPS value is clamped to at least `1` so invalid zero
    /// values cannot cause division by zero.
    pub fn record(
        &mut self,
        elapsed: Duration,
        configured_fps: u32,
    ) -> FpsPerformanceStatus {
        self.record_at(Instant::now(), elapsed, configured_fps)
    }

    fn record_at(
        &mut self,
        now: Instant,
        elapsed: Duration,
        configured_fps: u32,
    ) -> FpsPerformanceStatus {
        self.samples.push_back((now, elapsed));
        self.total += elapsed;

        while let Some((timestamp, duration)) = self.samples.front().copied() {
            if now.duration_since(timestamp) <= FPS_AVERAGE_WINDOW {
                break;
            }

            self.samples.pop_front();
            self.total = self.total.saturating_sub(duration);
        }

        self.status(configured_fps)
    }

    /// Returns the current status without recording another frame.
    pub fn status(&self, configured_fps: u32) -> FpsPerformanceStatus {
        let sample_count = self.samples.len();

        if sample_count == 0 {
            return FpsPerformanceStatus::default();
        }

        let average_seconds =
            self.total.as_secs_f64() / sample_count as f64;

        let average_frame_time =
            Duration::from_secs_f64(average_seconds);

        let safe_configured_fps = configured_fps.max(1);
        let ideal_seconds = 1.0 / safe_configured_fps as f64;

        let warning_state = if average_seconds > ideal_seconds * 2.0 {
            FpsWarningState::Critical
        } else if average_seconds > ideal_seconds * 1.5 {
            FpsWarningState::Warning
        } else {
            FpsWarningState::Normal
        };

        let average_fps = if average_seconds > 0.0 {
            (1.0 / average_seconds)
                .round()
                .clamp(0.0, u32::MAX as f64) as u32
        } else {
            0
        };

        FpsPerformanceStatus {
            average_fps,
            average_frame_time,
            warning_state,
            sample_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_window_is_normal() {
        let window = FrameTimeWindow::new();
        let status = window.status(30);

        assert_eq!(status.warning_state, FpsWarningState::Normal);
        assert_eq!(status.average_fps, 0);
        assert_eq!(status.sample_count, 0);
    }

    #[test]
    fn normal_frame_time_is_reported_as_normal() {
        let mut window = FrameTimeWindow::new();
        let status = window.record(Duration::from_millis(30), 30);

        assert_eq!(status.warning_state, FpsWarningState::Normal);
        assert_eq!(status.average_fps, 33);
    }

    #[test]
    fn slow_frame_time_is_reported_as_warning() {
        let mut window = FrameTimeWindow::new();
        let status = window.record(Duration::from_millis(55), 30);

        assert_eq!(status.warning_state, FpsWarningState::Warning);
        assert_eq!(status.average_fps, 18);
    }

    #[test]
    fn very_slow_frame_time_is_reported_as_critical() {
        let mut window = FrameTimeWindow::new();
        let status = window.record(Duration::from_millis(70), 30);

        assert_eq!(status.warning_state, FpsWarningState::Critical);
        assert_eq!(status.average_fps, 14);
    }

    #[test]
    fn clear_removes_all_samples() {
        let mut window = FrameTimeWindow::new();
        window.record(Duration::from_millis(40), 30);

        window.clear();

        assert!(window.is_empty());
        assert_eq!(window.sample_count(), 0);
        assert_eq!(
            window.status(30).warning_state,
            FpsWarningState::Normal,
        );
    }

    #[test]
    fn zero_configured_fps_is_safely_clamped() {
        let mut window = FrameTimeWindow::new();
        let status = window.record(Duration::from_millis(10), 0);

        assert_eq!(status.warning_state, FpsWarningState::Normal);
    }
}

