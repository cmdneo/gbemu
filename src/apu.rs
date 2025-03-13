//! Audio Procrssing Unit
mod audio;
mod noise;
mod parts;
mod pulse;
mod wave;

use std::sync::mpsc;

use noise::NoiseChannel;
use pulse::PulseChannel;
use wave::WaveChannel;

use crate::{info, log, regs};

/// Audio Processing Unit, generates samples and sends it to the
/// audio player(backend).  
/// I cannot believe that this works... :').
pub(crate) struct Apu {
    pub(crate) nr52: regs::AudioNr52,
    pub(crate) nr51: regs::AudioNr51,
    pub(crate) nr50: regs::AudioNr50,

    pub(crate) ch1: PulseChannel,
    pub(crate) ch2: PulseChannel,
    pub(crate) ch3: WaveChannel,
    pub(crate) ch4: NoiseChannel,

    // For seeing channel response.
    pub(crate) ch_avgs: [f64; 4],

    player: Option<audio::AudioPlayer>,
    sender: mpsc::Sender<audio::TimedSample>,

    // For audio timing and sample generation.
    time_elapsed: f64,
    last_sampled: f64,
    sample_rate: f64,

    // For the HPF(high pass filter) to eliminate any DC offset.
    charge_factor: f64,
    left_charge: f64,
    right_charge: f64,
}

impl Apu {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let player = match audio::AudioPlayer::new(rx) {
            Ok(p) => Some(p),
            Err(e) => {
                log::error(&format!("apu: cannot open audio device: {}", e));
                None
            }
        };

        Self {
            ch1: PulseChannel::new(true),
            ch2: PulseChannel::new(false),
            ch3: WaveChannel::new(),
            ch4: NoiseChannel::new(),

            nr52: Default::default(),
            nr51: Default::default(),
            nr50: Default::default(),

            ch_avgs: Default::default(),

            sender: tx,
            player,
            last_sampled: 0.0,
            time_elapsed: 0.0,
            sample_rate: 0.0,

            charge_factor: 0.0,
            left_charge: 0.0,
            right_charge: 0.0,
        }
    }

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

        // TODO Optimize this, this is really slow.
        self.ch1.tick(dots);
        self.ch2.tick(dots);
        self.ch3.tick(dots);
        self.ch4.tick(dots);

        self.nr52.ch1_on = self.ch1.on as u8;
        self.nr52.ch2_on = self.ch2.on as u8;
        self.nr52.ch3_on = self.ch3.on as u8;
        self.nr52.ch4_on = self.ch4.on as u8;

        self.time_elapsed += 1.0 / info::FREQUENCY as f64 * dots as f64;

        if self.sample_rate != 0.0 {
            self.add_audio_sample();
        }
    }

    pub(crate) fn play_audio(&mut self) {
        if let Some(p) = self.player.as_mut() {
            // HACK - Sample at a higher rate as samples produced are less
            // than actual rate for some reason which causes
            // audio buffer underruns. A factor of +10% seems to work.
            self.sample_rate = p.sample_rate() as f64 * 1.1;
            self.charge_factor = 0.999958_f64.powf(info::FREQUENCY as f64 / self.sample_rate);
            self.left_charge = 0.0;
            self.right_charge = 0.0;

            p.control(audio::Message::Play);
        } else {
            log::error("apu: cannot play audio: no audio player");
        }
    }

    pub(crate) fn pause_audio(&mut self) {
        if let Some(p) = self.player.as_mut() {
            p.control(audio::Message::Pause);
        }
    }

    pub(crate) fn stop_audio(&mut self) {
        if let Some(p) = self.player.as_mut() {
            p.control(audio::Message::Stop);
        }

        self.player = None;
    }

    fn add_audio_sample(&mut self) {
        if self.time_elapsed - self.last_sampled < 1.0 / self.sample_rate {
            return;
        }

        // In range [-4, 4].
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

        (lv, rv) = self.apply_high_pass(lv, rv);

        let s = audio::TimedSample {
            left: (lv / 4.0) as f32,
            right: (rv / 4.0) as f32,
            timestamp: self.time_elapsed,
        };

        self.last_sampled = self.time_elapsed;
        self.sender.send(s).unwrap();
        self.update_channel_avgs(v1, v2, v3, v4);
    }

    fn apply_high_pass(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        let out_l = in_l - self.left_charge;
        self.left_charge = in_l - out_l * self.charge_factor;

        let out_r = in_r - self.right_charge;
        self.right_charge = in_r - out_r * self.charge_factor;

        (out_l, out_r)
    }

    fn update_channel_avgs(&mut self, c1: f64, c2: f64, c3: f64, c4: f64) {
        const F: f64 = 0.1;

        self.ch_avgs[0] = c1.abs() * F + self.ch_avgs[0] * (1.0 - F);
        self.ch_avgs[1] = c2.abs() * F + self.ch_avgs[1] * (1.0 - F);
        self.ch_avgs[2] = c3.abs() * F + self.ch_avgs[2] * (1.0 - F);
        self.ch_avgs[3] = c4.abs() * F + self.ch_avgs[3] * (1.0 - F);
    }
}

#[inline(always)]
fn d_to_a(enabled: bool, d: u8) -> f64 {
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
