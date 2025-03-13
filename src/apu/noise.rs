use crate::{counter::Counter, regs};

use super::parts;

#[derive(Default)]
pub(crate) struct NoiseChannel {
    pub(crate) on: bool,
    pub(crate) output: u8,

    pub(crate) n41: regs::AudioNx1,
    pub(crate) n42: regs::AudioNx2,
    pub(crate) n44: regs::AudioNx4,
    n43: regs::AudioN43, // for detecting writes easily

    lsfr_bits: u16,
    lsft_ctr: Counter,

    envelope: parts::VolumeEnvelope,
    length_timer: parts::LengthTimer,
}

impl NoiseChannel {
    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub(crate) fn apu_tick(&mut self) {
        if self.n44.length_timer_enable == 1 {
            self.length_timer.tick();
            self.on = self.length_timer.is_active();
        }

        if !self.dac_enabled() {
            self.on = false;
        }

        self.envelope.tick();
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        if self.n44.trigger == 1 {
            self.trigger();
            return;
        }

        if self.lsft_ctr.tick(dots) > 0 {
            let out = !((self.lsfr_bits >> 1) ^ self.lsfr_bits) & 1;

            set_bit(&mut self.lsfr_bits, 15, out == 1);
            if self.n43.lfsr_width == 1 {
                set_bit(&mut self.lsfr_bits, 7, out == 1);
            }

            self.output = (self.lsfr_bits & 1) as u8 * self.envelope.volume();
            self.lsfr_bits >>= 1;
        }
    }

    pub(crate) fn read_n43(&self) -> u8 {
        self.n43.read()
    }

    pub(crate) fn write_n43(&mut self, v: u8) {
        self.n43.write(v);
        self.lsft_ctr = parts::new_lfsr_counter(&self.n43);
    }

    fn trigger(&mut self) {
        self.n44.trigger = 0;
        if !self.dac_enabled() {
            return;
        }

        self.on = true;
        self.envelope.setup(&self.n42);

        if !self.length_timer.is_active() {
            self.length_timer.setup(false, self.n41.length_period);
        }
    }

    fn dac_enabled(&self) -> bool {
        !(self.n42.direction == 0 && self.n42.initial_volume == 0)
    }
}

fn set_bit(bits: &mut u16, index: u16, value: bool) {
    if value {
        *bits |= 1 << index
    } else {
        *bits &= !(1 << index)
    }
}
