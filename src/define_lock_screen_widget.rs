//! define_lock_screen_widget.rs
//!
//! Central definition of the Screenshaver lock-screen authentication widget.
//! Runtime state and OpenGL drawing remain in lock_screen_widget.rs; this module
//! owns the widget's default appearance and timing parameters so future TOML
//! configuration can be introduced without duplicating design constants.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct LockScreenWidgetConfig {
    pub parent_radius: f32,
    pub child_radius: f32,
    pub background_radius: f32,

    pub child_inactive_color: [f32; 4],
    pub child_active_color: [f32; 4],
    pub child_error_color: [f32; 4],
    pub background_color: [f32; 4],
    pub halo_color: [f32; 4],

    pub halo_strength: f32,
    pub randomize_child_display: bool,

    pub child_active_fade_time: Duration,
    pub authentication_failure_duration: Duration,
}

impl Default for LockScreenWidgetConfig {
    fn default() -> Self {
        Self {
            parent_radius: 130.0,
            child_radius: 24.0,
            background_radius: 180.0,

            child_inactive_color: [
                0.02,
                0.02,
                0.02,
                1.00,
            ],

            child_active_color: [
                1.00,
                0.6470588,
                0.0,
                1.00,
            ],

            child_error_color: [
                0.95,
                0.10,
                0.10,
                1.00,
            ],

            background_color: [
                0.02,
                0.02,
                0.02,
                1.00,
            ],

            halo_color: [
                1.00,
                1.00,
                0.00,
                1.00,
            ],

            halo_strength: 0.08,

            // Sequential clockwise display is the Screenshaver default.
            // This may later be exposed through screenshaver.toml.
            randomize_child_display: false,

            child_active_fade_time:
                Duration::from_millis(300),

            authentication_failure_duration:
                Duration::from_secs(2),
        }
    }
}
