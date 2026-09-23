use rustfft::{
    num_complex::Complex,
    Fft,
    FftPlanner,
};

use std::sync::{Arc, OnceLock, RwLock};
use std::time::{
    Duration,
    Instant,
};


const FFT_SIZE: usize = 2048;

const FFT_HOP_SIZE: usize = FFT_SIZE / 2;

const BASS_MIN_HZ: f32 = 20.0;
const BASS_MAX_HZ: f32 = 250.0;

const MID_MIN_HZ: f32 = 250.0;
const MID_MAX_HZ: f32 = 4_000.0;

const TREBLE_MIN_HZ: f32 = 4_000.0;
const TREBLE_MAX_HZ: f32 = 20_000.0;

const DOMINANT_MIN_HZ: f32 = 40.0;
const DOMINANT_MAX_HZ: f32 = 12_000.0;

// Experimental Audio Motion spectrum.  This is deliberately independent of
// the three broad Audio Bloom bands.  Both complementary measurements are
// derived from the same FFT that is already performed for Audio Bloom.
const AUDIO_MOTION_VOCAL_MIN_HZ: f32 = 1_000.0;
const AUDIO_MOTION_VOCAL_MAX_HZ: f32 = 3_000.0;
const AUDIO_MOTION_MIN_HZ: f32 = 20.0;
const AUDIO_MOTION_MAX_HZ: f32 = 20_000.0;

// Spectral Bloom has its own analysis state so none of these
// constants affect the existing bass/midrange/treble envelopes
// used by Audio Bloom.
const SPECTRAL_BUCKET_COUNT: usize = 48;

const SPECTRAL_REFERENCE_ATTACK: f32 = 0.08;
const SPECTRAL_REFERENCE_RELEASE: f32 = 0.004;
const SPECTRAL_REFERENCE_FLOOR: f32 = 0.000_001;

const SPECTRAL_ENVELOPE_ATTACK: f32 = 0.45;
const SPECTRAL_ENVELOPE_RELEASE: f32 = 0.04;

const SPECTRAL_NORMALIZED_TARGET: f32 = 0.65;

// A challenger must exceed the currently selected bucket by
// roughly 12 percent before the dominant color is allowed to move.
const SPECTRAL_DOMINANCE_HYSTERESIS: f32 = 1.12;

// Once a Spectral color wins, keep it for at least 125 ms so the
// visual result is readable and inversion/rotation changes remain
// perceptible.  A dramatically stronger challenger may still break
// the hold early.
const SPECTRAL_COLOR_HOLD: Duration =
    Duration::from_millis(125);

const SPECTRAL_HOLD_BREAK_MULTIPLIER: f32 = 1.75;

// Ignore extremely weak spectral bins relative to the strongest
// bucket in the current FFT window.  This prevents quiet high-band
// noise from winning merely because its adaptive reference is small.
const SPECTRAL_RELATIVE_FLOOR: f32 = 0.0025;

const SPECTRAL_FREQUENCY_SMOOTHING: f32 = 0.28;

// Loudness Bloom measures short-term stereo RMS in dBFS, then maps it
// relative to a slowly adapting recent-peak reference.  This keeps the
// full red-to-violet range usable across different playback-monitor gains
// without allowing a quiet passage to immediately redefine itself as loud.
const LOUDNESS_RMS_FLOOR: f32 = 0.000_001;

// Number of decibels below the recent peak that map to the red end of
// the Loudness Bloom color scale.  The recent peak itself maps to violet.
const LOUDNESS_DYNAMIC_RANGE_DB: f32 = 18.0;

// The peak reference follows genuinely louder material quickly but falls
// very slowly so ordinary quiet sections remain visually quiet.
const LOUDNESS_REFERENCE_ATTACK: f32 = 0.60;
const LOUDNESS_REFERENCE_RELEASE: f32 = 0.0025;

// Expand the quiet/low-level portion of the color range.  A value
// greater than 1.0 pulls ordinary musical levels downward toward
// red/orange while preserving 0.0 = red and 1.0 = violet.
const LOUDNESS_COLOR_CURVE_EXPONENT: f32 = 2.00;

const LOUDNESS_ENVELOPE_ATTACK: f32 = 0.45;
const LOUDNESS_ENVELOPE_RELEASE: f32 = 0.035;

const REPORT_INTERVAL:
    Duration =
    Duration::from_secs(1);

const REFERENCE_ATTACK:
    f32 =
    0.08;

const REFERENCE_RELEASE:
    f32 =
    0.004;

const REFERENCE_FLOOR:
    f32 =
    0.000_001;

const NORMALIZED_TARGET:
    f32 =
    0.65;

const ENVELOPE_ATTACK:
    f32 =
    0.35;

const ENVELOPE_RELEASE:
    f32 =
    0.08;


// Experimental vocal detector used by --test-audio-motion.
//
// Keep this analysis independent from Audio Bloom, Spectral Bloom, and
// Loudness Bloom.  These measurements describe how voice-like the current
// FFT window appears; they must not gate bass or treble motion.
const VOCAL_F0_MIN_HZ: f32 = 80.0;
const VOCAL_F0_MAX_HZ: f32 = 1_000.0;
const VOCAL_HARMONIC_MAX_HZ: f32 = 4_000.0;
const VOCAL_HARMONIC_TOLERANCE_BINS: usize = 1;
const VOCAL_MAX_HARMONICS: usize = 12;

const VOCAL_FLATNESS_MIN_HZ: f32 = 250.0;
const VOCAL_FLATNESS_MAX_HZ: f32 = 4_000.0;
const VOCAL_FLATNESS_FLOOR: f32 = 0.000_000_000_001;

const VOCAL_FORMANT1_MIN_HZ: f32 = 250.0;
const VOCAL_FORMANT1_MAX_HZ: f32 = 1_000.0;
const VOCAL_FORMANT2_MIN_HZ: f32 = 800.0;
const VOCAL_FORMANT2_MAX_HZ: f32 = 2_500.0;
const VOCAL_FORMANT3_MIN_HZ: f32 = 1_500.0;
const VOCAL_FORMANT3_MAX_HZ: f32 = 3_500.0;

const VOCAL_PERSISTENCE_ATTACK: f32 = 0.22;
const VOCAL_PERSISTENCE_RELEASE: f32 = 0.10;

const VOCAL_ACTIVE_ON_THRESHOLD: f32 = 0.64;
const VOCAL_ACTIVE_OFF_THRESHOLD: f32 = 0.44;
const VOCAL_ACTIVE_ON_WINDOWS: u32 = 4;
const VOCAL_ACTIVE_OFF_WINDOWS: u32 = 7;


pub struct AudioAnalyzer {

    sample_rate:
        f32,

    samples:
        Vec<f32>,

    // Per-frame stereo power used only by Loudness Bloom.  Keep this
    // separate from the mono FFT sample stream so left/right phase
    // cancellation cannot artificially suppress apparent loudness.
    loudness_power_samples:
        Vec<f32>,

    fft:
        Arc<dyn Fft<f32>>,

    fft_buffer:
        Vec<Complex<f32>>,

    last_report:
        Instant,

    reference:
        AudioBands,

    smoothed:
        AudioBands,

    spectral_reference:
        [f32; SPECTRAL_BUCKET_COUNT],

    spectral_smoothed:
        [f32; SPECTRAL_BUCKET_COUNT],

    spectral_dominant_bucket:
        Option<usize>,

    spectral_dominant_since:
        Option<Instant>,

    spectral_dominant_frequency_hz:
        f32,

    loudness_smoothed:
        f32,

    loudness_reference_dbfs:
        Option<f32>,

    // Experimental Audio Motion spectral state.  These references and
    // envelopes are separate from Audio Bloom so this experiment cannot
    // change the behavior of the production three-band effects.
    audio_motion_inside_reference:
        f32,

    audio_motion_outside_reference:
        f32,

    audio_motion_inside_smoothed:
        f32,

    audio_motion_outside_smoothed:
        f32,

    // Experimental vocal-classification state.  This is deliberately
    // separate from the existing band/reference/envelope state.
    vocal_previous_fundamental_hz:
        f32,

    vocal_persistence:
        f32,

    vocal_active:
        bool,

    vocal_on_windows:
        u32,

    vocal_off_windows:
        u32,

    vocal_previous_formants_hz:
        [f32; 3],

    vocal_formant_motion:
        f32,
}


impl AudioAnalyzer {

    pub fn new(
        sample_rate: u32,
    ) -> Self {

        let mut planner =
            FftPlanner::<f32>::new();


        let fft =
            planner.plan_fft_forward(
                FFT_SIZE
            );


        Self {
            sample_rate:
                sample_rate as f32,

            samples:
                Vec::with_capacity(
                    FFT_SIZE * 2
                ),

            loudness_power_samples:
                Vec::with_capacity(
                    FFT_SIZE * 2
                ),

            fft,

            fft_buffer:
                vec![
                    Complex::new(
                        0.0,
                        0.0,
                    );
                    FFT_SIZE
                ],

            last_report:
                Instant::now(),

            reference:
                AudioBands::default(),

            smoothed:
                AudioBands::default(),

            spectral_reference:
                [0.0; SPECTRAL_BUCKET_COUNT],

            spectral_smoothed:
                [0.0; SPECTRAL_BUCKET_COUNT],

            spectral_dominant_bucket:
                None,

            spectral_dominant_since:
                None,

            spectral_dominant_frequency_hz:
                0.0,

            loudness_smoothed:
                0.0,

            loudness_reference_dbfs:
                None,

            audio_motion_inside_reference:
                0.0,

            audio_motion_outside_reference:
                0.0,

            audio_motion_inside_smoothed:
                0.0,

            audio_motion_outside_smoothed:
                0.0,

            vocal_previous_fundamental_hz:
                0.0,

            vocal_persistence:
                0.0,

            vocal_active:
                false,

            vocal_on_windows:
                0,

            vocal_off_windows:
                0,

            vocal_previous_formants_hz:
                [0.0; 3],

            vocal_formant_motion:
                0.0,
        }
    }


    pub fn push_s16_stereo_bytes(
        &mut self,
        data: &[u8],
    ) -> Option<AudioBands> {

        for frame in
            data.chunks_exact(4)
        {

            let left =
                i16::from_ne_bytes(
                    [
                        frame[0],
                        frame[1],
                    ]
                ) as f32
                / 32768.0;


            let right =
                i16::from_ne_bytes(
                    [
                        frame[2],
                        frame[3],
                    ]
                ) as f32
                / 32768.0;


            self.samples.push(
                (left + right)
                    * 0.5
            );

            self.loudness_power_samples.push(
                (
                    left * left
                        + right * right
                )
                    * 0.5
            );
        }


        let mut latest =
            None;


        while self.samples.len()
            >= FFT_SIZE
        {

            let raw =
                self.analyze_window();


            let normalized =
                self.equalize(
                    raw
                );


            let smoothed =
                self.smooth(
                    normalized
                );


            latest =
                Some(
                    smoothed
                );


            if self.last_report.elapsed()
                >= REPORT_INTERVAL
            {

                println!(
                    "[AUDIO] Spectrum raw:    bass={:.6} mid={:.6} treble={:.6}",
                    raw.bass,
                    raw.midrange,
                    raw.treble,
                );

                println!(
                    "[AUDIO] Spectrum norm:   bass={:.3} mid={:.3} treble={:.3}",
                    normalized.bass,
                    normalized.midrange,
                    normalized.treble,
                );

                println!(
                    "[AUDIO] Spectrum smooth: bass={:.3} mid={:.3} treble={:.3} dominant={:.1} Hz level={:.3}",
                    smoothed.bass,
                    smoothed.midrange,
                    smoothed.treble,
                    smoothed.dominant_frequency_hz,
                    smoothed.loudness,
                );

                println!(
                    "[AUDIO] Vocal diagnostic v2: active={} likelihood={:.3} f0={:.1} Hz harmonicity={:.3} flatness={:.3} formant_pattern={:.3} persistence={:.3}",
                    if smoothed.vocal_active { "YES" } else { "NO" },
                    smoothed.vocal_likelihood,
                    smoothed.vocal_fundamental_hz,
                    smoothed.vocal_harmonicity,
                    smoothed.vocal_flatness,
                    smoothed.vocal_formant_score,
                    smoothed.vocal_persistence,
                );


                self.last_report =
                    Instant::now();
            }


            self.samples.drain(
                ..FFT_HOP_SIZE
            );

            self.loudness_power_samples.drain(
                ..FFT_HOP_SIZE
            );
        }


        latest
    }


    fn equalize(
        &mut self,
        raw: AudioBands,
    ) -> AudioBands {

        self.reference.bass =
            update_reference(
                self.reference.bass,
                raw.bass,
            );

        self.reference.midrange =
            update_reference(
                self.reference.midrange,
                raw.midrange,
            );

        self.reference.treble =
            update_reference(
                self.reference.treble,
                raw.treble,
            );

        AudioBands {
            bass:
                normalize_band(
                    raw.bass,
                    self.reference.bass,
                ),

            midrange:
                normalize_band(
                    raw.midrange,
                    self.reference.midrange,
                ),

            treble:
                normalize_band(
                    raw.treble,
                    self.reference.treble,
                ),

            dominant_frequency_hz:
                raw.dominant_frequency_hz,

            loudness:
                raw.loudness,

            vocal_likelihood:
                raw.vocal_likelihood,

            vocal_fundamental_hz:
                raw.vocal_fundamental_hz,

            vocal_harmonicity:
                raw.vocal_harmonicity,

            vocal_flatness:
                raw.vocal_flatness,

            vocal_formant_score:
                raw.vocal_formant_score,

            vocal_persistence:
                raw.vocal_persistence,

            vocal_active:
                raw.vocal_active,
        }
    }


    fn smooth(
        &mut self,
        normalized: AudioBands,
    ) -> AudioBands {

        self.smoothed.bass =
            smooth_band(
                self.smoothed.bass,
                normalized.bass,
            );

        self.smoothed.midrange =
            smooth_band(
                self.smoothed.midrange,
                normalized.midrange,
            );

        self.smoothed.treble =
            smooth_band(
                self.smoothed.treble,
                normalized.treble,
            );

        self.loudness_smoothed =
            smooth_loudness(
                self.loudness_smoothed,
                normalized.loudness,
            );

        self.smoothed.loudness =
            self.loudness_smoothed;

        // Vocal analysis already has its own temporal persistence and
        // hysteresis.  Do not run it through the Audio Bloom band smoother.
        self.smoothed.vocal_likelihood =
            normalized.vocal_likelihood;
        self.smoothed.vocal_fundamental_hz =
            normalized.vocal_fundamental_hz;
        self.smoothed.vocal_harmonicity =
            normalized.vocal_harmonicity;
        self.smoothed.vocal_flatness =
            normalized.vocal_flatness;
        self.smoothed.vocal_formant_score =
            normalized.vocal_formant_score;
        self.smoothed.vocal_persistence =
            normalized.vocal_persistence;
        self.smoothed.vocal_active =
            normalized.vocal_active;

        let overall_energy =
            self.smoothed.bass
                .max(
                    self.smoothed.midrange
                )
                .max(
                    self.smoothed.treble
                );

        if overall_energy > 0.0001
            && normalized.dominant_frequency_hz > 0.0
        {
            // Spectral-only bucket analysis already applies its own
            // attack/release, hysteresis, and logarithmic frequency
            // smoothing.  Carry that result through unchanged here.
            self.smoothed.dominant_frequency_hz =
                normalized.dominant_frequency_hz;
        } else if overall_energy <= 0.0001 {
            self.smoothed.dominant_frequency_hz =
                0.0;
        }


        self.smoothed
    }


    fn analyze_window(
        &mut self,
    ) -> AudioBands {

        // Loudness is measured from the two captured channels before the
        // mono FFT downmix.  Using (left + right) / 2 for RMS can suppress
        // stereo material when the channels contain phase differences.
        let loudness_power_sum =
            self.loudness_power_samples
                .iter()
                .take(
                    FFT_SIZE
                )
                .copied()
                .sum::<f32>();


        let rms =
            (
                loudness_power_sum
                    / FFT_SIZE as f32
            )
                .sqrt();


        let loudness =
            self.rms_to_relative_loudness_level(
                rms
            );


        let denominator =
            (FFT_SIZE - 1) as f32;


        for index in 0..FFT_SIZE {

            let phase =
                2.0
                * std::f32::consts::PI
                * index as f32
                / denominator;


            let hann =
                0.5
                * (
                    1.0
                    - phase.cos()
                );


            self.fft_buffer[index] =
                Complex::new(
                    self.samples[index]
                        * hann,
                    0.0,
                );
        }


        self.fft.process(
            &mut self.fft_buffer
        );


        let mut bass_power =
            0.0_f32;

        let mut bass_bins =
            0_u32;

        let mut mid_power =
            0.0_f32;

        let mut mid_bins =
            0_u32;

        let mut treble_power =
            0.0_f32;

        let mut treble_bins =
            0_u32;

        let mut audio_motion_inside_power =
            0.0_f32;

        let mut audio_motion_inside_bins =
            0_u32;

        let mut audio_motion_outside_power =
            0.0_f32;

        let mut audio_motion_outside_bins =
            0_u32;

        let mut spectral_power =
            [0.0_f32; SPECTRAL_BUCKET_COUNT];

        let mut spectral_bins =
            [0_u32; SPECTRAL_BUCKET_COUNT];

        for bin in
            1..=(FFT_SIZE / 2)
        {

            let frequency =
                bin as f32
                * self.sample_rate
                / FFT_SIZE as f32;


            let value =
                self.fft_buffer[bin];


            let power =
                value.re
                    * value.re
                + value.im
                    * value.im;


            if frequency >= AUDIO_MOTION_MIN_HZ
                && frequency <= AUDIO_MOTION_MAX_HZ
            {
                if frequency >= AUDIO_MOTION_VOCAL_MIN_HZ
                    && frequency <= AUDIO_MOTION_VOCAL_MAX_HZ
                {
                    audio_motion_inside_power += power;
                    audio_motion_inside_bins += 1;
                } else {
                    audio_motion_outside_power += power;
                    audio_motion_outside_bins += 1;
                }
            }


            if frequency >= DOMINANT_MIN_HZ
                && frequency <= DOMINANT_MAX_HZ
            {
                let bucket =
                    spectral_bucket_index(
                        frequency
                    );

                spectral_power[bucket] +=
                    power;

                spectral_bins[bucket] +=
                    1;
            }


            if frequency >= BASS_MIN_HZ
                && frequency < BASS_MAX_HZ
            {

                bass_power +=
                    power;

                bass_bins +=
                    1;
            }
            else if frequency >= MID_MIN_HZ
                && frequency < MID_MAX_HZ
            {

                mid_power +=
                    power;

                mid_bins +=
                    1;
            }
            else if frequency >= TREBLE_MIN_HZ
                && frequency <= TREBLE_MAX_HZ
            {

                treble_power +=
                    power;

                treble_bins +=
                    1;
            }
        }


        self.update_audio_motion_spectrum(
            band_rms(audio_motion_inside_power, audio_motion_inside_bins),
            band_rms(audio_motion_outside_power, audio_motion_outside_bins),
        );


        let dominant_frequency_hz =
            self.update_spectral_dominant(
                &spectral_power,
                &spectral_bins,
            );

        let vocal =
            self.analyze_vocal_window();


        AudioBands {
            bass:
                band_rms(
                    bass_power,
                    bass_bins,
                ),

            midrange:
                band_rms(
                    mid_power,
                    mid_bins,
                ),

            treble:
                band_rms(
                    treble_power,
                    treble_bins,
                ),

            dominant_frequency_hz,

            loudness,

            vocal_likelihood:
                vocal.likelihood,

            vocal_fundamental_hz:
                vocal.fundamental_hz,

            vocal_harmonicity:
                vocal.harmonicity,

            vocal_flatness:
                vocal.flatness,

            vocal_formant_score:
                vocal.formant_score,

            vocal_persistence:
                vocal.persistence,

            vocal_active:
                vocal.active,
        }
    }


    fn update_audio_motion_spectrum(
        &mut self,
        inside_raw: f32,
        outside_raw: f32,
    ) {
        self.audio_motion_inside_reference = update_reference(
            self.audio_motion_inside_reference,
            inside_raw,
        );
        self.audio_motion_outside_reference = update_reference(
            self.audio_motion_outside_reference,
            outside_raw,
        );

        let inside_normalized = normalize_band(
            inside_raw,
            self.audio_motion_inside_reference,
        );
        let outside_normalized = normalize_band(
            outside_raw,
            self.audio_motion_outside_reference,
        );

        self.audio_motion_inside_smoothed = smooth_band(
            self.audio_motion_inside_smoothed,
            inside_normalized,
        );
        self.audio_motion_outside_smoothed = smooth_band(
            self.audio_motion_outside_smoothed,
            outside_normalized,
        );

        if let Ok(mut spectrum) = shared_audio_motion_spectrum().write() {
            *spectrum = AudioMotionSpectrum {
                inside_1k_3k: self.audio_motion_inside_smoothed,
                outside_1k_3k: self.audio_motion_outside_smoothed,
            };
        }
    }


    fn analyze_vocal_window(
        &mut self,
    ) -> VocalAnalysis {

        let nyquist_bin =
            FFT_SIZE / 2;

        let mut log_power_sum =
            0.0_f32;

        let mut flatness_power_sum =
            0.0_f32;

        let mut flatness_bins =
            0_u32;

        for bin in 1..=nyquist_bin {

            let frequency =
                bin as f32
                    * self.sample_rate
                    / FFT_SIZE as f32;

            if frequency < VOCAL_FLATNESS_MIN_HZ
                || frequency > VOCAL_FLATNESS_MAX_HZ
            {
                continue;
            }

            let value =
                self.fft_buffer[bin];

            let power =
                value.re * value.re
                    + value.im * value.im;

            let safe_power =
                power.max(
                    VOCAL_FLATNESS_FLOOR
                );

            log_power_sum +=
                safe_power.ln();

            flatness_power_sum +=
                safe_power;

            flatness_bins +=
                1;
        }

        let flatness =
            if flatness_bins > 0
                && flatness_power_sum > 0.0
            {
                let geometric_mean =
                    (
                        log_power_sum
                            / flatness_bins as f32
                    )
                        .exp();

                let arithmetic_mean =
                    flatness_power_sum
                        / flatness_bins as f32;

                (
                    geometric_mean
                        / arithmetic_mean.max(
                            VOCAL_FLATNESS_FLOOR
                        )
                )
                    .clamp(
                        0.0,
                        1.0,
                    )
            } else {
                1.0
            };

        let (
            fundamental_hz,
            harmonicity,
        ) =
            estimate_vocal_harmonicity(
                &self.fft_buffer,
                self.sample_rate,
            );

        // Unlike the first experiment, this score does not award points
        // merely because energy exists inside broad "formant" frequency
        // ranges.  It searches for distinct, locally prominent resonances.
        let (
            formant_score,
            formants_hz,
        ) =
            estimate_vocal_formant_pattern(
                &self.fft_buffer,
                self.sample_rate,
            );

        let mut comparable_formants =
            0_u32;

        let mut movement_sum =
            0.0_f32;

        for index in 0..3 {

            let current =
                formants_hz[index];

            let previous =
                self.vocal_previous_formants_hz[index];

            if current > 0.0
                && previous > 0.0
            {
                let relative_change =
                    (
                        current
                            - previous
                    )
                        .abs()
                        / previous.max(
                            1.0
                        );

                // Human vowel resonances tend to move, but not teleport.
                // Very static resonances and enormous one-frame jumps both
                // receive less credit.
                let motion =
                    if relative_change < 0.008 {
                        relative_change
                            / 0.008
                            * 0.35
                    } else if relative_change <= 0.18 {
                        0.35
                            + (
                                relative_change
                                    - 0.008
                            )
                                / (
                                    0.18
                                        - 0.008
                                )
                                * 0.65
                    } else if relative_change <= 0.45 {
                        1.0
                            - (
                                relative_change
                                    - 0.18
                            )
                                / (
                                    0.45
                                        - 0.18
                                )
                                * 0.75
                    } else {
                        0.10
                    };

                movement_sum +=
                    motion.clamp(
                        0.0,
                        1.0,
                    );

                comparable_formants +=
                    1;
            }

            if current > 0.0 {
                self.vocal_previous_formants_hz[index] =
                    current;
            }
        }

        let formant_motion_target =
            if comparable_formants > 0 {
                movement_sum
                    / comparable_formants as f32
            } else {
                0.0
            };

        let formant_motion_rate =
            if formant_motion_target
                > self.vocal_formant_motion
            {
                0.18
            } else {
                0.08
            };

        self.vocal_formant_motion =
            (
                self.vocal_formant_motion
                    + (
                        formant_motion_target
                            - self.vocal_formant_motion
                    )
                        * formant_motion_rate
            )
                .clamp(
                    0.0,
                    1.0,
                );

        let pitch_persistence =
            if fundamental_hz > 0.0
                && self.vocal_previous_fundamental_hz > 0.0
            {
                let octave_distance =
                    (
                        fundamental_hz
                            / self.vocal_previous_fundamental_hz
                    )
                        .log2()
                        .abs();

                (
                    1.0
                        - octave_distance
                            / 0.35
                )
                    .clamp(
                        0.0,
                        1.0,
                    )
            } else if fundamental_hz > 0.0 {
                0.35
            } else {
                0.0
            };

        if fundamental_hz > 0.0 {
            self.vocal_previous_fundamental_hz =
                fundamental_hz;
        }

        // Persistence now requires both harmonic continuity and some
        // plausible formant movement.  This deliberately reduces the
        // reward given to stable instrumental harmonic structures.
        let persistence_target =
            (
                0.30 * pitch_persistence
                    + 0.25 * harmonicity
                    + 0.45 * self.vocal_formant_motion
            )
                .clamp(
                    0.0,
                    1.0,
                );

        let persistence_rate =
            if persistence_target
                > self.vocal_persistence
            {
                VOCAL_PERSISTENCE_ATTACK
            } else {
                VOCAL_PERSISTENCE_RELEASE
            };

        self.vocal_persistence =
            (
                self.vocal_persistence
                    + (
                        persistence_target
                            - self.vocal_persistence
                    )
                        * persistence_rate
            )
                .clamp(
                    0.0,
                    1.0,
                );

        // Flatness remains only weak evidence for vocal classification.
        // It never gates bass or high-frequency Audio Motion response.
        let tonal_score =
            (
                1.0
                    - flatness
            )
                .clamp(
                    0.0,
                    1.0,
                );

        // Version 2 deliberately makes distinct formant structure the
        // strongest requirement.  Harmonicity alone can no longer make an
        // instrumental passage look strongly vocal.
        let raw_likelihood =
            (
                0.18 * harmonicity
                    + 0.42 * formant_score
                    + 0.25 * self.vocal_persistence
                    + 0.10 * self.vocal_formant_motion
                    + 0.05 * tonal_score
            )
                .clamp(
                    0.0,
                    1.0,
                );

        if self.vocal_active {

            if raw_likelihood
                <= VOCAL_ACTIVE_OFF_THRESHOLD
            {
                self.vocal_off_windows =
                    self.vocal_off_windows
                        .saturating_add(
                            1
                        );
            } else {
                self.vocal_off_windows =
                    0;
            }

            self.vocal_on_windows =
                0;

            if self.vocal_off_windows
                >= VOCAL_ACTIVE_OFF_WINDOWS
            {
                self.vocal_active =
                    false;

                self.vocal_off_windows =
                    0;
            }
        } else {

            if raw_likelihood
                >= VOCAL_ACTIVE_ON_THRESHOLD
            {
                self.vocal_on_windows =
                    self.vocal_on_windows
                        .saturating_add(
                            1
                        );
            } else {
                self.vocal_on_windows =
                    0;
            }

            self.vocal_off_windows =
                0;

            if self.vocal_on_windows
                >= VOCAL_ACTIVE_ON_WINDOWS
            {
                self.vocal_active =
                    true;

                self.vocal_on_windows =
                    0;
            }
        }

        VocalAnalysis {
            likelihood:
                raw_likelihood,

            fundamental_hz,

            harmonicity,

            flatness,

            formant_score,

            persistence:
                self.vocal_persistence,

            active:
                self.vocal_active,
        }
    }

    fn rms_to_relative_loudness_level(
        &mut self,
        rms: f32,
    ) -> f32 {

        if rms <= LOUDNESS_RMS_FLOOR {
            return 0.0;
        }


        let dbfs =
            20.0
                * rms
                    .max(
                        LOUDNESS_RMS_FLOOR
                    )
                    .log10();


        let reference =
            match self.loudness_reference_dbfs {

                Some(current_reference) => {

                    let rate =
                        if dbfs > current_reference {
                            LOUDNESS_REFERENCE_ATTACK
                        } else {
                            LOUDNESS_REFERENCE_RELEASE
                        };


                    current_reference
                        + (
                            dbfs
                                - current_reference
                        )
                            * rate
                }

                None => {
                    dbfs
                }
            };


        self.loudness_reference_dbfs =
            Some(
                reference
            );


        let quiet_dbfs =
            reference
                - LOUDNESS_DYNAMIC_RANGE_DB;


        let linear_level =
            (
                (
                    dbfs
                        - quiet_dbfs
                )
                    / LOUDNESS_DYNAMIC_RANGE_DB
            )
                .clamp(
                    0.0,
                    1.0,
                );


        linear_level
            .powf(
                LOUDNESS_COLOR_CURVE_EXPONENT
            )
    }


    fn update_spectral_dominant(
        &mut self,
        bucket_power: &[f32; SPECTRAL_BUCKET_COUNT],
        bucket_bins: &[u32; SPECTRAL_BUCKET_COUNT],
    ) -> f32 {

        let mut raw_level =
            [0.0_f32; SPECTRAL_BUCKET_COUNT];

        let mut strongest_raw =
            0.0_f32;


        for bucket in 0..SPECTRAL_BUCKET_COUNT {

            raw_level[bucket] =
                band_rms(
                    bucket_power[bucket],
                    bucket_bins[bucket],
                );

            strongest_raw =
                strongest_raw.max(
                    raw_level[bucket]
                );
        }


        if strongest_raw <= SPECTRAL_REFERENCE_FLOOR {

            for bucket in 0..SPECTRAL_BUCKET_COUNT {
                self.spectral_smoothed[bucket] =
                    smooth_spectral_bucket(
                        self.spectral_smoothed[bucket],
                        0.0,
                    );
            }

            return self.spectral_dominant_frequency_hz;
        }


        let absolute_floor =
            strongest_raw
                * SPECTRAL_RELATIVE_FLOOR;


        for bucket in 0..SPECTRAL_BUCKET_COUNT {

            let sample =
                raw_level[bucket];


            self.spectral_reference[bucket] =
                update_spectral_reference(
                    self.spectral_reference[bucket],
                    sample,
                );


            let normalized =
                if sample >= absolute_floor
                    && bucket_bins[bucket] > 0
                {
                    normalize_spectral_bucket(
                        sample,
                        self.spectral_reference[bucket],
                    )
                } else {
                    0.0
                };


            self.spectral_smoothed[bucket] =
                smooth_spectral_bucket(
                    self.spectral_smoothed[bucket],
                    normalized,
                );
        }


        let mut challenger_bucket =
            0_usize;

        let mut challenger_score =
            0.0_f32;


        for bucket in 0..SPECTRAL_BUCKET_COUNT {

            let relative_energy =
                (
                    raw_level[bucket]
                    / strongest_raw
                )
                    .clamp(
                        0.0,
                        1.0,
                    )
                    .sqrt();


            // Adaptive normalization is the primary score.  A small
            // real-energy term breaks ties without allowing the usual
            // bass-heavy spectral tilt to dominate again.
            let score =
                self.spectral_smoothed[bucket]
                    * (
                        0.85
                        + 0.15
                            * relative_energy
                    );


            if score > challenger_score {

                challenger_score =
                    score;

                challenger_bucket =
                    bucket;
            }
        }


        let now =
            Instant::now();


        let selected_bucket =
            match self.spectral_dominant_bucket {

                Some(current_bucket) => {

                    let current_relative_energy =
                        (
                            raw_level[current_bucket]
                            / strongest_raw
                        )
                            .clamp(
                                0.0,
                                1.0,
                            )
                            .sqrt();


                    let current_score =
                        self.spectral_smoothed[current_bucket]
                            * (
                                0.85
                                + 0.15
                                    * current_relative_energy
                            );


                    let hold_active =
                        self.spectral_dominant_since
                            .map(
                                |since| {
                                    now.duration_since(
                                        since
                                    ) < SPECTRAL_COLOR_HOLD
                                }
                            )
                            .unwrap_or(
                                false
                            );


                    let strong_break =
                        challenger_bucket != current_bucket
                            && challenger_score
                                > current_score
                                    * SPECTRAL_HOLD_BREAK_MULTIPLIER;


                    let normal_switch =
                        challenger_bucket != current_bucket
                            && challenger_score
                                > current_score
                                    * SPECTRAL_DOMINANCE_HYSTERESIS;


                    if strong_break
                        || (
                            !hold_active
                            && normal_switch
                        )
                    {
                        challenger_bucket
                    } else {
                        current_bucket
                    }
                }

                None =>
                    challenger_bucket,
            };


        if self.spectral_dominant_bucket
            != Some(
                selected_bucket
            )
        {
            self.spectral_dominant_since =
                Some(
                    now
                );
        } else if self.spectral_dominant_since
            .is_none()
        {
            self.spectral_dominant_since =
                Some(
                    now
                );
        }


        self.spectral_dominant_bucket =
            Some(
                selected_bucket
            );


        let target_frequency =
            spectral_bucket_center_hz(
                selected_bucket
            );


        self.spectral_dominant_frequency_hz =
            smooth_spectral_frequency(
                self.spectral_dominant_frequency_hz,
                target_frequency,
            );


        self.spectral_dominant_frequency_hz
    }
}


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
)]
pub struct AudioMotionSpectrum {
    // Normalized/smoothed energy inside the experimental 1-3 kHz vocal band.
    pub inside_1k_3k: f32,

    // Normalized/smoothed energy from 20 Hz-20 kHz excluding 1-3 kHz.
    pub outside_1k_3k: f32,
}


pub fn shared_audio_motion_spectrum() -> &'static RwLock<AudioMotionSpectrum> {
    static SHARED: OnceLock<RwLock<AudioMotionSpectrum>> = OnceLock::new();
    SHARED.get_or_init(|| RwLock::new(AudioMotionSpectrum::default()))
}


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
)]
pub struct AudioBands {

    pub bass:
        f32,

    pub midrange:
        f32,

    pub treble:
        f32,

    pub dominant_frequency_hz:
        f32,

    pub loudness:
        f32,

    // Experimental voice-like spectral measurements.  Existing audio
    // effects may ignore these fields; --test-audio-motion consumes them.
    pub vocal_likelihood:
        f32,

    pub vocal_fundamental_hz:
        f32,

    pub vocal_harmonicity:
        f32,

    pub vocal_flatness:
        f32,

    pub vocal_formant_score:
        f32,

    pub vocal_persistence:
        f32,

    pub vocal_active:
        bool,
}


#[derive(
    Clone,
    Copy,
    Debug,
    Default,
)]
struct VocalAnalysis {

    likelihood:
        f32,

    fundamental_hz:
        f32,

    harmonicity:
        f32,

    flatness:
        f32,

    formant_score:
        f32,

    persistence:
        f32,

    active:
        bool,
}


fn estimate_vocal_formant_pattern(
    spectrum: &[Complex<f32>],
    sample_rate: f32,
) -> (
    f32,
    [f32; 3],
) {

    const MIN_HZ: f32 = 250.0;
    const MAX_HZ: f32 = 3_500.0;
    const SMOOTH_RADIUS: usize = 2;
    const MIN_PEAK_SEPARATION_HZ: f32 = 180.0;

    let min_bin =
        (
            MIN_HZ
                * FFT_SIZE as f32
                / sample_rate
        )
            .ceil()
            .max(
                1.0
            ) as usize;

    let max_bin =
        (
            MAX_HZ
                * FFT_SIZE as f32
                / sample_rate
        )
            .floor()
            .min(
                (FFT_SIZE / 2 - 1) as f32
            ) as usize;

    if min_bin + 2 >= max_bin {
        return (
            0.0,
            [0.0; 3],
        );
    }

    let mut envelope =
        vec![
            0.0_f32;
            max_bin + 1
        ];

    for bin in min_bin..=max_bin {

        let start =
            bin.saturating_sub(
                SMOOTH_RADIUS
            )
                .max(
                    min_bin
                );

        let end =
            (
                bin
                    + SMOOTH_RADIUS
            )
                .min(
                    max_bin
                );

        let mut sum =
            0.0_f32;

        let mut count =
            0_u32;

        for sample_bin in start..=end {

            let value =
                spectrum[sample_bin];

            sum +=
                (
                    value.re * value.re
                        + value.im * value.im
                )
                    .sqrt();

            count +=
                1;
        }

        envelope[bin] =
            if count > 0 {
                sum
                    / count as f32
            } else {
                0.0
            };
    }

    let mut candidates:
        Vec<(
            usize,
            f32,
        )> =
        Vec::new();

    for bin in (min_bin + 1)..max_bin {

        let value =
            envelope[bin];

        if value <= envelope[bin - 1]
            || value < envelope[bin + 1]
        {
            continue;
        }

        let shoulder =
            5_usize;

        let left =
            bin.saturating_sub(
                shoulder
            )
                .max(
                    min_bin
                );

        let right =
            (
                bin
                    + shoulder
            )
                .min(
                    max_bin
                );

        let baseline =
            0.5
                * (
                    envelope[left]
                        + envelope[right]
                );

        let prominence =
            if value > 0.0 {
                (
                    (
                        value
                            - baseline
                    )
                        / value
                )
                    .clamp(
                        0.0,
                        1.0,
                    )
            } else {
                0.0
            };

        if prominence >= 0.08 {
            candidates.push(
                (
                    bin,
                    prominence,
                )
            );
        }
    }

    candidates.sort_by(
        |a, b| {
            b.1
                .partial_cmp(
                    &a.1
                )
                .unwrap_or(
                    std::cmp::Ordering::Equal
                )
        }
    );

    let mut selected:
        Vec<(
            usize,
            f32,
        )> =
        Vec::new();

    for candidate in candidates {

        let frequency =
            candidate.0 as f32
                * sample_rate
                / FFT_SIZE as f32;

        let separated =
            selected
                .iter()
                .all(
                    |existing| {
                        let existing_frequency =
                            existing.0 as f32
                                * sample_rate
                                / FFT_SIZE as f32;

                        (
                            frequency
                                - existing_frequency
                        )
                            .abs()
                            >= MIN_PEAK_SEPARATION_HZ
                    }
                );

        if separated {
            selected.push(
                candidate
            );
        }

        if selected.len() >= 6 {
            break;
        }
    }

    selected.sort_by_key(
        |entry| entry.0
    );

    let best_in_range =
        |low_hz: f32,
         high_hz: f32|
         -> Option<(
             f32,
             f32,
         )> {

            selected
                .iter()
                .filter_map(
                    |(
                        bin,
                        prominence,
                    )| {
                        let frequency =
                            *bin as f32
                                * sample_rate
                                / FFT_SIZE as f32;

                        if frequency >= low_hz
                            && frequency <= high_hz
                        {
                            Some(
                                (
                                    frequency,
                                    *prominence,
                                )
                            )
                        } else {
                            None
                        }
                    }
                )
                .max_by(
                    |a, b| {
                        a.1
                            .partial_cmp(
                                &b.1
                            )
                            .unwrap_or(
                                std::cmp::Ordering::Equal
                            )
                    }
                )
        };

    let f1 =
        best_in_range(
            VOCAL_FORMANT1_MIN_HZ,
            VOCAL_FORMANT1_MAX_HZ,
        );

    let f2 =
        best_in_range(
            VOCAL_FORMANT2_MIN_HZ,
            VOCAL_FORMANT2_MAX_HZ,
        );

    let f3 =
        best_in_range(
            VOCAL_FORMANT3_MIN_HZ,
            VOCAL_FORMANT3_MAX_HZ,
        );

    let mut formants =
        [0.0_f32; 3];

    let mut prominence_sum =
        0.0_f32;

    let mut found =
        0_u32;

    for (
        index,
        formant,
    ) in [
        f1,
        f2,
        f3,
    ]
        .into_iter()
        .enumerate()
    {
        if let Some(
            (
                frequency,
                prominence,
            )
        ) = formant
        {
            formants[index] =
                frequency;

            prominence_sum +=
                prominence;

            found +=
                1;
        }
    }

    let coverage =
        found as f32
            / 3.0;

    let average_prominence =
        if found > 0 {
            prominence_sum
                / found as f32
        } else {
            0.0
        };

    let spacing_score =
        if formants[0] > 0.0
            && formants[1] > 0.0
            && formants[2] > 0.0
        {
            let d12 =
                formants[1]
                    - formants[0];

            let d23 =
                formants[2]
                    - formants[1];

            let ordered =
                d12 > 180.0
                    && d23 > 180.0;

            if ordered {
                (
                    0.5
                        * (
                            d12
                                / 900.0
                        )
                            .clamp(
                                0.0,
                                1.0,
                            )
                        + 0.5
                            * (
                                d23
                                    / 1_300.0
                            )
                                .clamp(
                                    0.0,
                                    1.0,
                                )
                )
                    .clamp(
                        0.0,
                        1.0,
                    )
            } else {
                0.0
            }
        } else {
            0.0
        };

    let score =
        (
            0.50 * coverage
                + 0.35 * average_prominence
                + 0.15 * spacing_score
        )
            .clamp(
                0.0,
                1.0,
            );

    (
        score,
        formants,
    )
}


fn estimate_vocal_harmonicity(
    spectrum: &[Complex<f32>],
    sample_rate: f32,
) -> (
    f32,
    f32,
) {

    let min_bin =
        (
            VOCAL_F0_MIN_HZ
                * FFT_SIZE as f32
                / sample_rate
        )
            .ceil()
            .max(
                1.0
            ) as usize;

    let max_bin =
        (
            VOCAL_F0_MAX_HZ
                * FFT_SIZE as f32
                / sample_rate
        )
            .floor()
            .min(
                (FFT_SIZE / 2) as f32
            ) as usize;

    let harmonic_limit_bin =
        (
            VOCAL_HARMONIC_MAX_HZ
                * FFT_SIZE as f32
                / sample_rate
        )
            .floor()
            .min(
                (FFT_SIZE / 2) as f32
            ) as usize;

    if min_bin >= max_bin
        || harmonic_limit_bin <= min_bin
    {
        return (
            0.0,
            0.0,
        );
    }

    let mut total_power =
        0.0_f32;

    for bin in min_bin..=harmonic_limit_bin {

        let value =
            spectrum[bin];

        total_power +=
            value.re * value.re
                + value.im * value.im;
    }

    if total_power
        <= VOCAL_FLATNESS_FLOOR
    {
        return (
            0.0,
            0.0,
        );
    }

    let mut best_bin =
        0_usize;

    let mut best_score =
        0.0_f32;

    for candidate_bin in min_bin..=max_bin {

        let mut harmonic_power =
            0.0_f32;

        let mut harmonic_weight =
            0.0_f32;

        for harmonic in 1..=VOCAL_MAX_HARMONICS {

            let center_bin =
                candidate_bin
                    .saturating_mul(
                        harmonic
                    );

            if center_bin
                > harmonic_limit_bin
            {
                break;
            }

            let start_bin =
                center_bin.saturating_sub(
                    VOCAL_HARMONIC_TOLERANCE_BINS
                )
                    .max(
                        1
                    );

            let end_bin =
                (
                    center_bin
                        + VOCAL_HARMONIC_TOLERANCE_BINS
                )
                    .min(
                        harmonic_limit_bin
                    );

            let mut local_peak =
                0.0_f32;

            for bin in start_bin..=end_bin {

                let value =
                    spectrum[bin];

                let power =
                    value.re * value.re
                        + value.im * value.im;

                local_peak =
                    local_peak.max(
                        power
                    );
            }

            let weight =
                1.0
                    / (
                        harmonic as f32
                    )
                        .sqrt();

            harmonic_power +=
                local_peak
                    * weight;

            harmonic_weight +=
                weight;
        }

        if harmonic_weight <= 0.0 {
            continue;
        }

        let score =
            (
                harmonic_power
                    / harmonic_weight
            )
                / (
                    total_power
                        / (
                            harmonic_limit_bin
                                - min_bin
                                + 1
                        ) as f32
                )
                    .max(
                        VOCAL_FLATNESS_FLOOR
                    );

        if score > best_score {
            best_score =
                score;

            best_bin =
                candidate_bin;
        }
    }

    if best_bin == 0 {
        return (
            0.0,
            0.0,
        );
    }

    let fundamental_hz =
        best_bin as f32
            * sample_rate
            / FFT_SIZE as f32;

    // The raw harmonic-comb score is intentionally compressed into a
    // bounded diagnostic quantity.  Threshold tuning belongs to the
    // experiment rather than to existing Audio Bloom behavior.
    let harmonicity =
        (
            best_score
                / (
                    best_score
                        + 6.0
                )
        )
            .clamp(
                0.0,
                1.0,
            );

    (
        fundamental_hz,
        harmonicity,
    )
}


fn smooth_loudness(
    current: f32,
    target: f32,
) -> f32 {

    let rate =
        if target > current {
            LOUDNESS_ENVELOPE_ATTACK
        } else {
            LOUDNESS_ENVELOPE_RELEASE
        };

    (
        current
            + (
                target
                    - current
            )
                * rate
    )
        .clamp(
            0.0,
            1.0,
        )
}


fn spectral_bucket_index(
    frequency_hz: f32,
) -> usize {

    let position =
        (
            frequency_hz
                .clamp(
                    DOMINANT_MIN_HZ,
                    DOMINANT_MAX_HZ,
                )
                / DOMINANT_MIN_HZ
        )
            .ln()
        / (
            DOMINANT_MAX_HZ
                / DOMINANT_MIN_HZ
        )
            .ln();


    (
        position
            * SPECTRAL_BUCKET_COUNT as f32
    )
        .floor()
        .clamp(
            0.0,
            (SPECTRAL_BUCKET_COUNT - 1) as f32,
        ) as usize
}


fn spectral_bucket_center_hz(
    bucket: usize,
) -> f32 {

    let position =
        (
            bucket as f32
                + 0.5
        )
        / SPECTRAL_BUCKET_COUNT as f32;


    DOMINANT_MIN_HZ
        * (
            DOMINANT_MAX_HZ
                / DOMINANT_MIN_HZ
        )
            .powf(
                position
            )
}


fn update_spectral_reference(
    current: f32,
    sample: f32,
) -> f32 {

    let sample =
        sample.max(
            SPECTRAL_REFERENCE_FLOOR
        );


    if current <= 0.0 {
        return sample;
    }


    let rate =
        if sample > current {
            SPECTRAL_REFERENCE_ATTACK
        } else {
            SPECTRAL_REFERENCE_RELEASE
        };


    (
        current
            + (
                sample
                    - current
            )
                * rate
    )
        .max(
            SPECTRAL_REFERENCE_FLOOR
        )
}


fn normalize_spectral_bucket(
    sample: f32,
    reference: f32,
) -> f32 {

    if sample <= 0.0 {
        return 0.0;
    }


    (
        sample
            / reference.max(
                SPECTRAL_REFERENCE_FLOOR
            )
            * SPECTRAL_NORMALIZED_TARGET
    )
        .clamp(
            0.0,
            1.0,
        )
}


fn smooth_spectral_bucket(
    current: f32,
    target: f32,
) -> f32 {

    let rate =
        if target > current {
            SPECTRAL_ENVELOPE_ATTACK
        } else {
            SPECTRAL_ENVELOPE_RELEASE
        };


    (
        current
            + (
                target
                    - current
            )
                * rate
    )
        .clamp(
            0.0,
            1.0,
        )
}


fn smooth_spectral_frequency(
    current: f32,
    target: f32,
) -> f32 {

    if target <= 0.0 {
        return current;
    }


    if current <= 0.0 {
        return target;
    }


    let current_log =
        current.log2();

    let target_log =
        target.log2();


    (
        current_log
            + (
                target_log
                    - current_log
            )
                * SPECTRAL_FREQUENCY_SMOOTHING
    )
        .exp2()
}


fn smooth_band(
    current: f32,
    target: f32,
) -> f32 {

    let rate =
        if target > current {
            ENVELOPE_ATTACK
        }
        else {
            ENVELOPE_RELEASE
        };


    (
        current
        + (
            target
            - current
        )
        * rate
    )
        .clamp(
            0.0,
            1.0,
        )
}


fn update_reference(
    current: f32,
    sample: f32,
) -> f32 {

    let sample =
        sample.max(
            REFERENCE_FLOOR
        );


    if current <= 0.0 {
        return sample;
    }


    let rate =
        if sample > current {
            REFERENCE_ATTACK
        }
        else {
            REFERENCE_RELEASE
        };


    (
        current
        + (
            sample
            - current
        )
        * rate
    )
        .max(
            REFERENCE_FLOOR
        )
}


fn normalize_band(
    sample: f32,
    reference: f32,
) -> f32 {

    if sample <= 0.0 {
        return 0.0;
    }


    (
        sample
        / reference.max(
            REFERENCE_FLOOR
        )
        * NORMALIZED_TARGET
    )
        .clamp(
            0.0,
            1.0,
        )
}


fn band_rms(
    power: f32,
    bins: u32,
) -> f32 {

    if bins == 0 {
        return 0.0;
    }


    (
        power
        / bins as f32
    )
        .sqrt()
        / FFT_SIZE as f32
}
