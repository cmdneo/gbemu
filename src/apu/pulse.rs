use crate::{counter::Counter, regs};

use super::parts;

#[derive(Default)]
pub(crate) struct PulseChannel {
    pub(crate) on: bool,
    pub(crate) output: u8,

    pub(crate) nx0: regs::AudioNx0,
    pub(crate) nx1: regs::AudioNx1,
    pub(crate) nx2: regs::AudioNx2,
    pub(crate) nx3: regs::AudioNx3,
    pub(crate) nx4: regs::AudioNx4,

    /// Channel-1 has sweep and Channel-2 does not.
    use_sweep: bool,

    sweep_ctr: Counter,
    sweep_enabled: bool,
    shadow_period: u32,

    envelope: parts::VolumeEnvelope,
    length_timer: parts::LengthTimer,

    // Dot-tick driven
    divider: parts::PeriodDivider,
}

impl PulseChannel {
    pub(crate) fn new(use_sweep: bool) -> Self {
        Self {
            use_sweep,
            divider: parts::PeriodDivider::new(false),
            ..Default::default()
        }
    }

    pub(crate) fn apu_tick(&mut self) {
        // Writing 0 to sweep-pace pauses iterations.
        if self.sweep_enabled && self.nx0.pace != 0 {
            self.tick_sweep();
        }

        if self.nx4.length_timer_enable == 1 {
            self.length_timer.tick();
            self.on = self.length_timer.is_active();
        }

        if !self.dac_enabled() {
            self.on = false;
        }

        self.envelope.tick();
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        if self.nx4.trigger == 1 {
            self.trigger();
            return;
        }

        let s = parts::get_pwm_sample(self.nx1.wave_duty, self.divider.sample_idx());
        self.output = s * self.envelope.volume();

        self.divider.tick(dots);
        if self.divider.is_reload_allowed() {
            self.divider.update_period(&self.nx3, &self.nx4);
        }
    }

    fn trigger(&mut self) {
        self.nx4.trigger = 0;
        if !self.dac_enabled() {
            return;
        }

        self.on = true;
        self.divider.update_period(&self.nx3, &self.nx4);
        self.envelope = parts::VolumeEnvelope::new(&self.nx2);

        if !self.length_timer.is_active() {
            self.length_timer = parts::LengthTimer::new(false, self.nx1.length_period);
        }

        if self.use_sweep {
            self.setup_sweep();
        }
    }

    fn setup_sweep(&mut self) {
        self.shadow_period = self.divider.period();
        self.sweep_ctr = parts::new_period_sweep_counter(self.nx0.pace);
        self.sweep_enabled = self.nx0.pace != 0 || self.nx0.shift_step != 0;

        if self.nx0.shift_step == 0 {
            return;
        }

        let (_, ovf) = parts::calc_new_period(self.shadow_period, &self.nx0);
        self.on = !ovf;
    }

    fn tick_sweep(&mut self) {
        if self.sweep_ctr.tick(1) == 0 {
            return;
        }
        if self.nx0.shift_step == 0 {
            return;
        }

        let (new, ovf) = parts::calc_new_period(self.shadow_period, &self.nx0);
        if ovf {
            self.on = false;
            return;
        }

        self.shadow_period = new;
        parts::set_period(&mut self.nx3, &mut self.nx4, new);

        // Redo calculations but do not set period.
        let (_, ovf) = parts::calc_new_period(self.shadow_period, &self.nx0);
        self.on = !ovf;
    }

    fn dac_enabled(&self) -> bool {
        !(self.nx2.direction == 0 && self.nx2.initial_volume == 0)
    }
}
