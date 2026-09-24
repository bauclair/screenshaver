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
