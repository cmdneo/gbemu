use crate::{info, regs};

use super::parts;

#[derive(Default)]
pub(crate) struct WaveChannel {
    pub(crate) on: bool,
    pub(crate) output: u8,

    pub(crate) n30: regs::AudioN30,
    pub(crate) n31: regs::AudioN31,
    pub(crate) n32: regs::AudioN32,
    pub(crate) n33: regs::AudioNx3,
    pub(crate) n34: regs::AudioNx4,
    pub(crate) wave_ram: [u8; info::SIZE_AUDIO_WAVE_RAM],

    length_timer: parts::LengthTimer,
    divider: parts::PeriodDivider,
}

impl WaveChannel {
    pub(crate) fn new() -> Self {
        Self {
            divider: parts::PeriodDivider::new(true),
            ..Default::default()
        }
    }

    pub(crate) fn apu_tick(&mut self) {
        if self.n34.length_timer_enable == 1 {
            self.length_timer.tick();
            self.on = self.length_timer.is_active();
        }

        if self.n30.dac_on == 0 {
            self.on = false;
        }
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        if self.n34.trigger == 1 {
            self.trigger();
            return;
        }

        self.output = self.get_wave_sample();

        self.divider.tick(dots);
        if self.divider.is_reload_allowed() {
            self.divider.update_period(&self.n33, &self.n34);
        }
    }

    fn trigger(&mut self) {
        self.n34.trigger = 0;
        if self.n30.dac_on == 0 {
            return;
        }

        self.on = true;
        self.divider.update_period(&self.n33, &self.n34);

        if !self.length_timer.is_active() {
            self.length_timer = parts::LengthTimer::new(true, self.n31.length_period);
        }
    }

    #[inline]
    fn get_wave_sample(&self) -> u8 {
        let i = self.divider.sample_idx() as usize;
        let b = self.wave_ram[i / 2];
        let b = if i % 2 == 0 { b >> 4 } else { b & 0xF };

        match self.n32.output_level {
            0b00 => 0,
            0b01 => b,
            0b10 => b >> 1,
            0b11 => b >> 2,
            _ => unreachable!(),
        }
    }
}
