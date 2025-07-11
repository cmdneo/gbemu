use bincode::{Decode, Encode};

use crate::{
    counter::Counter,
    regs::{AudioN43, AudioNx0, AudioNx2, AudioNx3, AudioNx4},
};

const DIVIDER_MAX_PERIOD: u32 = 2048; // times 2(wave channel) or 4(others) dots
const LSFR_BASE_PERIOD: u32 = 16; // dots
const SWEEPER_BASE_PERIOD: u32 = 4; // APU-ticks
const LENGTH_BASE_PERIOD: u32 = 2; // APU-ticks
const ENVELOPE_BASE_PERIOD: u32 = 8; // APU-ticks
const PWM_WAVE_SAMPLES: [u8; 4] = [0b00000001, 0b00000011, 0b00001111, 0b00111111];

#[derive(Default, Encode, Decode)]
pub(crate) struct VolumeEnvelope {
    volume: u8,
    active: bool,
    decrement: bool,
    counter: Counter,
}

impl VolumeEnvelope {
    pub(crate) fn new(nx2: &AudioNx2) -> Self {
        assert!(nx2.pace <= 7);
        Self {
            volume: nx2.initial_volume,
            counter: Counter::new(ENVELOPE_BASE_PERIOD * nx2.pace as u32),
            decrement: nx2.direction == 0,
            active: nx2.pace != 0,
        }
    }

    pub(crate) fn tick(&mut self) {
        if !self.active || self.counter.tick(1) == 0 {
            return;
        }

        match (self.decrement, self.volume) {
            (true, 0) | (false, 15) => self.active = false,
            (true, _) => self.volume -= 1,
            (false, _) => self.volume += 1,
        }
    }

    #[inline]
    pub(crate) fn volume(&self) -> u8 {
        self.volume
    }
}

#[derive(Default, Encode, Decode)]
pub(crate) struct LengthTimer {
    active: bool,
    counter: Counter,
}

impl LengthTimer {
    pub(crate) fn new(is_wave_channel: bool, initial: u8) -> Self {
        let initial = initial as u32;
        let max_period = if is_wave_channel { 256 } else { 64 };

        assert!(initial < max_period);
        Self {
            counter: Counter::new(LENGTH_BASE_PERIOD * (max_period - initial)),
            active: true,
        }
    }

    pub(crate) fn tick(&mut self) {
        if self.active && self.counter.tick(1) > 0 {
            self.active = false;
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Default, Encode, Decode)]
pub(crate) struct PeriodDivider {
    wave_sample_count: u8,
    dots_per_tick: u32,

    counter: Counter,
    sample_idx: u8,
    period: u32,
    sample_finished: bool,
}

impl PeriodDivider {
    pub(crate) fn new(is_wave_channel: bool) -> Self {
        // Period divider ticks once every 2-dots for wave channel and
        // every 4-dots for other channels.
        let wave_sample_count = if is_wave_channel { 32 } else { 8 };
        let dots_per_tick = if is_wave_channel { 2 } else { 4 };

        Self {
            wave_sample_count,
            dots_per_tick,
            ..Default::default()
        }
    }

    pub(crate) fn update_period(&mut self, nx3: &AudioNx3, nx4: &AudioNx4) {
        let period = get_period(nx3, nx4);
        if self.period == period {
            return;
        }

        assert!(period <= DIVIDER_MAX_PERIOD);
        self.sample_idx = 0;
        self.period = get_period(nx3, nx4);
        self.counter = Counter::new(self.dots_per_tick * (DIVIDER_MAX_PERIOD - self.period));
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        self.sample_idx += self.counter.tick(dots) as u8;
        self.sample_finished = self.sample_idx >= self.wave_sample_count;
        self.sample_idx &= self.wave_sample_count - 1;
    }

    /// Returns true if wave_idx overflowed during the last tick.
    // Actual period is updated only after the sample finishes, that is,
    // wave counter wraps back to 0.
    pub(crate) fn is_reload_allowed(&self) -> bool {
        self.sample_finished
    }

    #[inline]
    pub(crate) fn period(&self) -> u32 {
        self.period
    }

    #[inline]
    pub(crate) fn sample_idx(&self) -> u8 {
        self.sample_idx
    }
}

pub(crate) fn new_lfsr_counter(n43: &AudioN43) -> Counter {
    // Period is: base_period * divider * 2 ^ shift, where
    // if divider is 0 then it is treated as 0.5.
    let fx = 1 << n43.clock_shift;
    let fx = if n43.clock_divider == 0 {
        fx / 2
    } else {
        fx * n43.clock_divider as u32
    };

    Counter::new(LSFR_BASE_PERIOD * fx)
}

pub(crate) fn new_period_sweep_counter(pace: u8) -> Counter {
    // Sweep timer treat a period of 0 as 8.
    assert!(pace <= 7);
    let pace = if pace == 0 { 8 } else { pace };

    Counter::new(SWEEPER_BASE_PERIOD * pace as u32)
}

pub(crate) fn calc_new_period(old_period: u32, nx0: &AudioNx0) -> (u32, bool) {
    let delta = old_period >> nx0.shift_step;
    let new_period = if nx0.direction == 0 {
        old_period + delta
    } else {
        old_period - delta
    };

    if new_period >= DIVIDER_MAX_PERIOD {
        (0, true)
    } else {
        (new_period, false)
    }
}

#[inline]
pub(crate) fn get_period(nx3: &AudioNx3, nx4: &AudioNx4) -> u32 {
    nx3.period_low as u32 | (nx4.period_high as u32) << 8
}

#[inline(always)]
pub(crate) fn get_pwm_sample(duty_cycle: u8, index: u8) -> u8 {
    (PWM_WAVE_SAMPLES[duty_cycle as usize] >> index) & 1
}

pub(crate) fn set_period(nx3: &mut AudioNx3, nx4: &mut AudioNx4, period: u32) {
    let low = period as u8;
    let high = (period >> 8) as u8;
    nx3.period_low = low;
    nx4.period_high = high;
}
