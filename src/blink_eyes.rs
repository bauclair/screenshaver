//! Independent blink scheduling for the animated Eyes texture.
//!
//! This module intentionally owns only blink state and timing.  It does not
//! draw pixels, upload OpenGL textures, or know anything about shader
//! Animation Speed.
//!
//! Design rules:
//!
//! - every visible eye has its own independently randomized blink deadline;
//! - blink timing is based on `std::time::Instant` (real elapsed time);
//! - shader animation speed never affects eye blinking;
//! - only one mutually-exclusive eye frame is active for an eye at a time;
//! - simultaneous blinking is bounded so the texture never performs an
//!   accidental synchronized "mass blink";
//! - blink frequency becomes sparser as eye density increases;
//! - blinking is disabled at 1024 requested eyes for the first experiment;
//! - two experimental blink sequences are available so visual testing can
//!   determine which looks better.

use std::time::{
    Duration,
    Instant,
};


// ============================================================
// Public animation types
// ============================================================

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub(crate) enum EyeFrame {
    Open,
    Half,
    Closed,
}


#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
pub(crate) enum BlinkSequence {

    /// OPEN -> HALF -> CLOSED -> HALF -> OPEN
    FullTransition,

    /// OPEN -> CLOSED -> HALF -> OPEN
    FastClose,
}


/// Experimental default.
///
/// Change this one constant while comparing the two blink behaviors.  Once a
/// preferred sequence is selected, the unused sequence can be removed.
pub(crate) const DEFAULT_BLINK_SEQUENCE:
    BlinkSequence =
    BlinkSequence::FullTransition;


// ============================================================
// Timing parameters
// ============================================================

/// OPEN -> HALF duration for the full-transition sequence.
const FULL_CLOSE_HALF_DURATION:
    Duration =
    Duration::from_millis(
        35
    );

/// HALF -> CLOSED duration for the full-transition sequence.
const FULL_CLOSE_CLOSED_DURATION:
    Duration =
    Duration::from_millis(
        35
    );

/// CLOSED hold time shared by both sequences.
const CLOSED_HOLD_DURATION:
    Duration =
    Duration::from_millis(
        35
    );

/// CLOSED -> HALF duration shared by both sequences.
const OPEN_HALF_DURATION:
    Duration =
    Duration::from_millis(
        40
    );

/// HALF -> OPEN duration shared by both sequences.
const OPEN_FINISH_DURATION:
    Duration =
    Duration::from_millis(
        45
    );

/// OPEN -> CLOSED duration for the fast-close sequence.
const FAST_CLOSE_DURATION:
    Duration =
    Duration::from_millis(
        55
    );

/// When the concurrency limit is already occupied, do not hammer the same
/// overdue eye every frame.  Postpone it by a short randomized delay.
const CONCURRENCY_RETRY_MIN:
    Duration =
    Duration::from_millis(
        120
    );

const CONCURRENCY_RETRY_MAX:
    Duration =
    Duration::from_millis(
        420
    );


// ============================================================
// Per-eye state
// ============================================================

#[derive(
    Debug,
    Clone,
)]
struct EyeBlinkState {

    frame:
        EyeFrame,

    blink_started:
        Option<Instant>,

    next_blink:
        Instant,
}


impl EyeBlinkState {

    fn new(
        next_blink: Instant,
    ) -> Self {

        Self {
            frame:
                EyeFrame::Open,

            blink_started:
                None,

            next_blink,
        }
    }


    fn is_blinking(
        &self,
    ) -> bool {

        self.blink_started
            .is_some()
    }
}


// ============================================================
// Blink controller
// ============================================================

pub(crate) struct BlinkController {

    eyes:
        Vec<EyeBlinkState>,

    requested_eye_count:
        usize,

    sequence:
        BlinkSequence,

    rng_state:
        u64,

    enabled:
        bool,

    maximum_simultaneous_blinks:
        usize,
}


impl BlinkController {

    /// Creates independent blink state for every eye instance in the rendered
    /// lattice.
    ///
    /// `eye_instance_count` is the number of actual visible/managed eye
    /// instances produced by `generate_eyes.rs`.  It need not exactly equal
    /// `requested_eye_count`, because staggered edge clipping can create a
    /// slightly different number of lattice instances.
    pub(crate) fn new(
        eye_instance_count: usize,
        requested_eye_count: usize,
        seed: u64,
        sequence: BlinkSequence,
        now: Instant,
    ) -> Self {

        let requested_eye_count =
            requested_eye_count.max(
                1
            );


        let (
            enabled,
            maximum_simultaneous_blinks,
        ) =
            density_policy(
                requested_eye_count
            );


        let mut controller =
            Self {
                eyes:
                    Vec::with_capacity(
                        eye_instance_count
                    ),

                requested_eye_count,

                sequence,

                rng_state:
                    normalize_seed(
                        seed
                    ),

                enabled,

                maximum_simultaneous_blinks,
            };


        //---------------------------------------------------------
        // Randomize initial deadlines across the full interval
        // rather than giving every eye the same startup phase.
        //---------------------------------------------------------

        for _ in
            0..eye_instance_count
        {
            let delay =
                controller.random_blink_delay(
                    true
                );


            controller.eyes.push(
                EyeBlinkState::new(
                    now + delay
                )
            );
        }


        controller
    }


    pub(crate) fn is_enabled(
        &self,
    ) -> bool {

        self.enabled
    }


    pub(crate) fn eye_count(
        &self,
    ) -> usize {

        self.eyes.len()
    }


    pub(crate) fn frame_for_eye(
        &self,
        eye_index: usize,
    ) -> EyeFrame {

        self.eyes
            .get(
                eye_index
            )
            .map(
                |eye| {
                    eye.frame
                }
            )
            .unwrap_or(
                EyeFrame::Open
            )
    }


    /// Advances blink timers from real elapsed time.
    ///
    /// Returns `true` only when at least one eye's displayed frame changed.
    /// The future renderer can use that signal to avoid rebuilding/uploading
    /// the Eyes texture when no visual blink state changed.
    pub(crate) fn update(
        &mut self,
        now: Instant,
    ) -> bool {

        if !self.enabled
            || self.eyes.is_empty()
        {
            return false;
        }


        let mut frame_changed =
            false;


        //---------------------------------------------------------
        // First advance blinks already in progress.  This frees
        // concurrency slots before overdue open eyes are considered.
        //---------------------------------------------------------

        for eye_index in
            0..self.eyes.len()
        {
            if self.eyes[
                eye_index
            ]
            .blink_started
            .is_none()
            {
                continue;
            }


            let previous_frame =
                self.eyes[
                    eye_index
                ]
                .frame;


            let finished =
                self.advance_active_blink(
                    eye_index,
                    now,
                );


            if self.eyes[
                eye_index
            ]
            .frame
                != previous_frame
            {
                frame_changed =
                    true;
            }


            if finished {
                let delay =
                    self.random_blink_delay(
                        false
                    );


                self.eyes[
                    eye_index
                ]
                .next_blink =
                    now + delay;
            }
        }


        //---------------------------------------------------------
        // Then begin independently scheduled new blinks, but never
        // exceed the density-specific concurrency limit.
        //---------------------------------------------------------

        let mut active_blinks =
            self.active_blink_count();


        for eye_index in
            0..self.eyes.len()
        {
            if active_blinks
                >= self.maximum_simultaneous_blinks
            {
                break;
            }


            if self.eyes[
                eye_index
            ]
            .is_blinking()
            {
                continue;
            }


            if now
                < self.eyes[
                    eye_index
                ]
                .next_blink
            {
                continue;
            }


            self.eyes[
                eye_index
            ]
            .blink_started =
                Some(
                    now
                );


            let initial_frame =
                match self.sequence {
                    BlinkSequence::FullTransition => {
                        EyeFrame::Half
                    }

                    BlinkSequence::FastClose => {
                        EyeFrame::Closed
                    }
                };


            if self.eyes[
                eye_index
            ]
            .frame
                != initial_frame
            {
                self.eyes[
                    eye_index
                ]
                .frame =
                    initial_frame;

                frame_changed =
                    true;
            }


            active_blinks +=
                1;
        }


        //---------------------------------------------------------
        // Any other overdue eye that could not begin because the
        // concurrency limit is full receives a short randomized
        // postponement.  That prevents a queued group from firing
        // together as soon as a slot opens.
        //---------------------------------------------------------

        if active_blinks
            >= self.maximum_simultaneous_blinks
        {
            for eye_index in
                0..self.eyes.len()
            {
                if self.eyes[
                    eye_index
                ]
                .is_blinking()
                {
                    continue;
                }


                if now
                    < self.eyes[
                        eye_index
                    ]
                    .next_blink
                {
                    continue;
                }


                let retry =
                    self.random_duration_between(
                        CONCURRENCY_RETRY_MIN,
                        CONCURRENCY_RETRY_MAX,
                    );


                self.eyes[
                    eye_index
                ]
                .next_blink =
                    now + retry;
            }
        }


        frame_changed
    }


    fn advance_active_blink(
        &mut self,
        eye_index: usize,
        now: Instant,
    ) -> bool {

        let Some(started) =
            self.eyes[
                eye_index
            ]
            .blink_started
        else {
            return false;
        };


        let elapsed =
            now.saturating_duration_since(
                started
            );


        let (
            frame,
            finished,
        ) =
            match self.sequence {
                BlinkSequence::FullTransition => {
                    full_transition_frame(
                        elapsed
                    )
                }

                BlinkSequence::FastClose => {
                    fast_close_frame(
                        elapsed
                    )
                }
            };


        self.eyes[
            eye_index
        ]
        .frame =
            frame;


        if finished {
            self.eyes[
                eye_index
            ]
            .blink_started =
                None;

            self.eyes[
                eye_index
            ]
            .frame =
                EyeFrame::Open;
        }


        finished
    }


    fn active_blink_count(
        &self,
    ) -> usize {

        self.eyes
            .iter()
            .filter(
                |eye| {
                    eye.is_blinking()
                }
            )
            .count()
    }


    fn random_blink_delay(
        &mut self,
        initial: bool,
    ) -> Duration {

        let (
            minimum,
            maximum,
        ) =
            blink_interval_for_density(
                self.requested_eye_count
            );


        if initial {
            //-------------------------------------------------
            // Startup distribution may begin anywhere from a
            // brief delay through the normal maximum interval.
            //-------------------------------------------------

            self.random_duration_between(
                Duration::from_millis(
                    7_000
                ),
                maximum,
            )

        } else {
            self.random_duration_between(
                minimum,
                maximum,
            )
        }
    }


    fn random_duration_between(
        &mut self,
        minimum: Duration,
        maximum: Duration,
    ) -> Duration {

        if maximum
            <= minimum
        {
            return minimum;
        }


        let minimum_millis =
            minimum.as_millis()
                .min(
                    u64::MAX as u128
                ) as u64;


        let maximum_millis =
            maximum.as_millis()
                .min(
                    u64::MAX as u128
                ) as u64;


        let span =
            maximum_millis
                .saturating_sub(
                    minimum_millis
                );


        let offset =
            if span == 0 {
                0
            } else {
                self.next_random_u64()
                    % (
                        span + 1
                    )
            };


        Duration::from_millis(
            minimum_millis
                + offset
        )
    }


    fn next_random_u64(
        &mut self,
    ) -> u64 {

        //---------------------------------------------------------
        // SplitMix64: tiny, deterministic, dependency-free PRNG.
        // Sufficient here because blink scheduling does not require
        // cryptographic randomness.
        //---------------------------------------------------------

        self.rng_state =
            self.rng_state
                .wrapping_add(
                    0x9E37_79B9_7F4A_7C15
                );


        let mut value =
            self.rng_state;


        value =
            (
                value
                    ^ (
                        value >> 30
                    )
            )
            .wrapping_mul(
                0xBF58_476D_1CE4_E5B9
            );


        value =
            (
                value
                    ^ (
                        value >> 27
                    )
            )
            .wrapping_mul(
                0x94D0_49BB_1331_11EB
            );


        value
            ^ (
                value >> 31
            )
    }
}


// ============================================================
// Blink sequences
// ============================================================

fn full_transition_frame(
    elapsed: Duration,
) -> (
    EyeFrame,
    bool,
) {

    let half_close_end =
        FULL_CLOSE_HALF_DURATION;


    let closed_start =
        half_close_end;


    let closed_end =
        closed_start
            + FULL_CLOSE_CLOSED_DURATION
            + CLOSED_HOLD_DURATION;


    let half_open_end =
        closed_end
            + OPEN_HALF_DURATION;


    let blink_end =
        half_open_end
            + OPEN_FINISH_DURATION;


    if elapsed
        < half_close_end
    {
        (
            EyeFrame::Half,
            false,
        )

    } else if elapsed
        < closed_end
    {
        (
            EyeFrame::Closed,
            false,
        )

    } else if elapsed
        < half_open_end
    {
        (
            EyeFrame::Half,
            false,
        )

    } else if elapsed
        < blink_end
    {
        (
            EyeFrame::Open,
            false,
        )

    } else {
        (
            EyeFrame::Open,
            true,
        )
    }
}


fn fast_close_frame(
    elapsed: Duration,
) -> (
    EyeFrame,
    bool,
) {

    let closed_end =
        FAST_CLOSE_DURATION
            + CLOSED_HOLD_DURATION;


    let half_open_end =
        closed_end
            + OPEN_HALF_DURATION;


    let blink_end =
        half_open_end
            + OPEN_FINISH_DURATION;


    if elapsed
        < closed_end
    {
        (
            EyeFrame::Closed,
            false,
        )

    } else if elapsed
        < half_open_end
    {
        (
            EyeFrame::Half,
            false,
        )

    } else if elapsed
        < blink_end
    {
        (
            EyeFrame::Open,
            false,
        )

    } else {
        (
            EyeFrame::Open,
            true,
        )
    }
}


// ============================================================
// Density policy
// ============================================================

fn density_policy(
    requested_eye_count: usize,
) -> (
    bool,
    usize,
) {

    match requested_eye_count {

        0..=32 => {
            (
                true,
                1,
            )
        }

        33..=128 => {
            (
                true,
                2,
            )
        }

        129..=256 => {
            (
                true,
                3,
            )
        }

        257..=512 => {
            (
                true,
                2,
            )
        }

        _ => {
            //-------------------------------------------------
            // First experimental policy: eyes:1024 is too dense
            // for an individual blink to remain visually useful.
            //-------------------------------------------------

            (
                false,
                0,
            )
        }
    }
}


fn blink_interval_for_density(
    requested_eye_count: usize,
) -> (
    Duration,
    Duration,
) {

    match requested_eye_count {

        0..=32 => {
            (
                Duration::from_millis(
                    30_000
                ),
                Duration::from_millis(
                    100_000
                ),
            )
        }

        33..=128 => {
            (
                Duration::from_millis(
                    40_000
                ),
                Duration::from_millis(
                    130_000
                ),
            )
        }

        129..=256 => {
            (
                Duration::from_millis(
                    56_000
                ),
                Duration::from_millis(
                    160_000
                ),
            )
        }

        257..=512 => {
            (
                Duration::from_millis(
                    90_000
                ),
                Duration::from_millis(
                    220_000
                ),
            )
        }

        _ => {
            (
                Duration::from_secs(
                    60
                ),
                Duration::from_secs(
                    60
                ),
            )
        }
    }
}


fn normalize_seed(
    seed: u64,
) -> u64 {

    if seed == 0 {
        0x6A09_E667_F3BC_C909
    } else {
        seed
    }
}


// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn high_density_disables_blinking() {

        let now =
            Instant::now();


        let controller =
            BlinkController::new(
                1024,
                1024,
                123,
                BlinkSequence::FullTransition,
                now,
            );


        assert!(
            !controller.is_enabled()
        );


        assert_eq!(
            controller.frame_for_eye(
                0
            ),
            EyeFrame::Open
        );
    }


    #[test]
    fn low_density_limits_concurrency_to_one() {

        let (
            enabled,
            maximum,
        ) =
            density_policy(
                16
            );


        assert!(
            enabled
        );


        assert_eq!(
            maximum,
            1
        );
    }


    #[test]
    fn medium_density_allows_limited_concurrency() {

        assert_eq!(
            density_policy(
                64
            ),
            (
                true,
                2,
            )
        );


        assert_eq!(
            density_policy(
                256
            ),
            (
                true,
                3,
            )
        );
    }


    #[test]
    fn full_transition_has_expected_frames() {

        assert_eq!(
            full_transition_frame(
                Duration::from_millis(
                    0
                )
            )
            .0,
            EyeFrame::Half
        );


        assert_eq!(
            full_transition_frame(
                Duration::from_millis(
                    80
                )
            )
            .0,
            EyeFrame::Closed
        );


        assert_eq!(
            full_transition_frame(
                Duration::from_millis(
                    120
                )
            )
            .0,
            EyeFrame::Half
        );
    }


    #[test]
    fn fast_close_skips_half_on_closing_side() {

        assert_eq!(
            fast_close_frame(
                Duration::from_millis(
                    0
                )
            )
            .0,
            EyeFrame::Closed
        );


        assert_eq!(
            fast_close_frame(
                Duration::from_millis(
                    100
                )
            )
            .0,
            EyeFrame::Half
        );
    }


    #[test]
    fn unavailable_eye_defaults_to_open() {

        let now =
            Instant::now();


        let controller =
            BlinkController::new(
                2,
                2,
                456,
                BlinkSequence::FullTransition,
                now,
            );


        assert_eq!(
            controller.frame_for_eye(
                999
            ),
            EyeFrame::Open
        );
    }
}
