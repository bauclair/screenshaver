use rustfft::{
    num_complex::Complex,
    Fft,
    FftPlanner,
};

use std::sync::Arc;
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
                    "[AUDIO] Spectrum smooth: bass={:.3} mid={:.3} treble={:.3}",
                    smoothed.bass,
                    smoothed.midrange,
                    smoothed.treble,
                );


                self.last_report =
                    Instant::now();
            }


            self.samples.drain(
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


        self.smoothed
    }


    fn analyze_window(
        &mut self,
    ) -> AudioBands {

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
        }
    }
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
