use crate::regs::TimerCtrl;

#[derive(Default)]
pub(crate) struct Timer {
    pub(crate) is_2x: bool,

    // Registers owned by it.
    pub(crate) tac: TimerCtrl,
    pub(crate) tma: u8,
    pub(crate) tima: u8,

    /// Internal 14-bit sys-clock incremented every M-cycle.
    sys_clock: u16,
    apu_event: bool,
    div_reset: bool,
    tima_overflowed: bool,
}

const SYS_CLOCK_MASK: u16 = !(!0 << 14);

impl Timer {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Update timers for new `sys_clock` value.
    /// Returns true if TIMER interrupt has been requested.
    pub(crate) fn tick(&mut self, mcycles: u16) -> bool {
        // DIV is either RESET or INCREMENTED.
        let mcycles = if self.div_reset {
            self.div_reset = false;
            mcycles - 1
        } else {
            mcycles
        };

        let mut timer_intr = false;

        for _ in 0..mcycles {
            let new = (self.sys_clock + 1) & SYS_CLOCK_MASK;

            timer_intr = self.tick_from_to(self.sys_clock, new) || timer_intr;
            self.sys_clock = new;
        }

        timer_intr
    }

    pub(crate) fn set_div(&mut self, _val: u8) {
        // setting DIV resets it to 0.
        self.sys_clock = 0;
        self.div_reset = true;
    }

    pub(crate) fn get_div(&self) -> u8 {
        (self.sys_clock >> 6) as u8
    }

    pub(crate) fn is_apu_event(&self) -> bool {
        self.apu_event
    }

    fn tick_from_to(&mut self, old: u16, new: u16) -> bool {
        let apu_idx = if self.is_2x { 11 } else { 10 };
        self.apu_event = has_fallen(old, new, apu_idx);

        if self.tac.enable == 0 {
            return false;
        }
        let timer_intr = if self.tima_overflowed {
            self.tima = self.tma;
            self.tima_overflowed = false;
            true
        } else {
            false
        };

        if !has_fallen(old, new, get_clock_fall_bit(self.tac.clock_select)) {
            return timer_intr;
        }

        // After TIMA overflows, the interrupt and loading TMA to TIMA
        // are delayed by one cycle and initially it holds 0.
        if self.tima == 0xFF {
            self.tima_overflowed = true;
            self.tima = 0;
        } else {
            self.tima += 1;
        }

        timer_intr
    }
}

/// Which bit of SYS_CLOCK should fall for TIMA to be incremented.
#[inline]
fn get_clock_fall_bit(clock_select: u8) -> u32 {
    match clock_select {
        1 => 1,
        2 => 3,
        3 => 5,
        0 => 7,
        _ => unreachable!(),
    }
}

#[inline]
fn has_fallen(old: u16, new: u16, fall_bit: u32) -> bool {
    (old >> fall_bit) & 1 == 1 && (new >> fall_bit) & 1 == 0
}
