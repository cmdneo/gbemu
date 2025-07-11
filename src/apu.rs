//! Audio Procrssing Unit

mod noise;
mod parts;
mod pulse;
mod wave;

use noise::NoiseChannel;
use pulse::PulseChannel;
use wave::WaveChannel;

use crate::{counter::Counter, regs};

/// Audio Processing Unit, generates samples and sends it to the
/// audio player(backend).  
/// I cannot believe that this works... :').
#[derive(bincode::Encode, bincode::Decode)]
pub(crate) struct Apu {
    #[bincode(with_serde)]
    pub(crate) nr52: regs::AudioNr52,
    #[bincode(with_serde)]
    pub(crate) nr51: regs::AudioNr51,
    #[bincode(with_serde)]
    pub(crate) nr50: regs::AudioNr50,

    pub(crate) ch1: PulseChannel,
    pub(crate) ch2: PulseChannel,
    pub(crate) ch3: WaveChannel,
    pub(crate) ch4: NoiseChannel,

    /// Audio samples in L R format.
    stereo_samples: Vec<f32>,
    sampling_counter: Counter,

    // For the HPF(high pass filter) to eliminate any DC offset.
    charge_factor: f64,
    left_charge: f64,
    right_charge: f64,
}

fn calc_charge_factor(period_in_dots: u32) -> f64 {
    // Constant calculated as specified in docs for high pass filter:
    // https://gbdev.io/pandocs/Audio_details.html
    0.999958_f64.powf(period_in_dots as f64)
}

impl Apu {
    pub(crate) fn new() -> Self {
        Self {
            ch1: PulseChannel::new(true),
            ch2: PulseChannel::new(false),
            ch3: WaveChannel::new(),
            ch4: NoiseChannel::new(),

            nr52: Default::default(),
            nr51: Default::default(),
            nr50: Default::default(),

            stereo_samples: Vec::new(),
            sampling_counter: Counter::new(0), // Start with sampling disabled

            charge_factor: 0.0,
            left_charge: 0.0,
            right_charge: 0.0,
        }
    }

    /// Tick for `dots` cycles. `apu_event` DIV-APU tick from the Timer.
    /// Ticks at normal speed even in dual-speed mode.
    pub(crate) fn tick(&mut self, dots: u32, apu_ticks: u8) {
        // DIV-APU counter ticks at only at 512Hz,
        // more that one tick in a single step means something is wrong.
        assert!(apu_ticks <= 1);

        for _ in 0..apu_ticks {
            self.ch1.apu_tick();
            self.ch2.apu_tick();
            self.ch3.apu_tick();
            self.ch4.apu_tick();
        }

        self.ch1.tick(dots);
        self.ch2.tick(dots);
        self.ch3.tick(dots);
        self.ch4.tick(dots);

        self.nr52.ch1_on = self.ch1.on as u8;
        self.nr52.ch2_on = self.ch2.on as u8;
        self.nr52.ch3_on = self.ch3.on as u8;
        self.nr52.ch4_on = self.ch4.on as u8;

        for _ in 0..self.sampling_counter.tick(dots) {
            self.add_audio_sample();
        }
    }

    /// Set sampling period and return previously accumulated samples,
    /// a period of 0 stops the sampling process.
    pub(crate) fn start_new_sampling(&mut self, period_in_dots: u32) -> Vec<f32> {
        self.sampling_counter = Counter::new(period_in_dots);
        self.charge_factor = calc_charge_factor(period_in_dots);

        std::mem::take(&mut self.stereo_samples)
    }

    fn add_audio_sample(&mut self) {
        // In range [-4, 4] for lv and rv amplitudes from all 4 channels combined.
        let mut lv = 0.0;
        let mut rv = 0.0;
        let mut add_lr = |left, right, v| {
            if left == 1 {
                lv += v;
            }
            if right == 1 {
                rv += v;
            }
        };

        let v1 = d_to_a(self.ch1.on, self.ch1.output);
        let v2 = d_to_a(self.ch2.on, self.ch2.output);
        let v3 = d_to_a(self.ch3.on, self.ch3.output);
        let v4 = d_to_a(self.ch4.on, self.ch4.output);

        add_lr(self.nr51.ch1_left, self.nr51.ch1_right, v1);
        add_lr(self.nr51.ch2_left, self.nr51.ch2_right, v2);
        add_lr(self.nr51.ch3_left, self.nr51.ch3_right, v3);
        add_lr(self.nr51.ch4_left, self.nr51.ch4_right, v4);

        lv = calc_sample_amp(self.nr50.vol_left, lv);
        rv = calc_sample_amp(self.nr50.vol_right, rv);
        (lv, rv) = self.apply_high_pass_filter(lv, rv);

        self.stereo_samples.push((lv / 4.0) as f32);
        self.stereo_samples.push((rv / 4.0) as f32);
    }

    fn apply_high_pass_filter(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        let out_l = in_l - self.left_charge;
        self.left_charge = in_l - out_l * self.charge_factor;

        let out_r = in_r - self.right_charge;
        self.right_charge = in_r - out_r * self.charge_factor;

        (out_l, out_r)
    }
}

/// Convert digital signal value {0..15} to analog signal value [-1, 1].
#[inline(always)]
fn d_to_a(enabled: bool, d: u8) -> f64 {
    // 7.5 is the mid point in 0-15.
    if enabled {
        (d as f64 - 7.5) / 7.5
    } else {
        0.0
    }
}

#[inline(always)]
fn calc_sample_amp(volume: u8, v: f64) -> f64 {
    v * (volume + 1) as f64 / 8.0
}
