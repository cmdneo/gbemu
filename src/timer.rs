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
    old_sys_clock: u16,
    apu_event: bool,
    tima_overflowed: bool,
}

const SYS_CLOCK_MASK: u16 = !(!0 << 14);

impl Timer {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn set_div(&mut self, _val: u8) {
        // setting DIV resets it to 0.
        self.sys_clock = 0;
    }

    pub(crate) fn get_div(&self) -> u8 {
        (self.sys_clock >> 6) as u8
    }

    /// Update timers for new `sys_clock` value.
    /// Returns true if TIMER interrupt has been requested.
    pub(crate) fn tick(&mut self, mcycles: u16) -> bool {
        assert!(mcycles > 0);
        let mut intr = false;
        self.apu_event = false;

        // DIV is not INCREMENTED if it is RESET in an mcycle.
        let mcycles = if self.sys_clock != self.old_sys_clock {
            intr = self.process_clock_tick();
            self.sys_clock = self.old_sys_clock;
            mcycles - 1
        } else {
            mcycles
        };

        for _ in 0..mcycles {
            self.sys_clock = (self.sys_clock + 1) & SYS_CLOCK_MASK;
            intr = self.process_clock_tick() || intr;
            self.old_sys_clock = self.sys_clock;
        }

        intr
    }

    fn process_clock_tick(&mut self) -> bool {
        let has_bit_fell = |bit: u32| {
            let old = (self.old_sys_clock >> bit) & 1;
            let new = (self.sys_clock >> bit) & 1;
            old == 1 && new == 0
        };

        let apu_idx = if self.is_2x { 11 } else { 10 };
        self.apu_event = has_bit_fell(apu_idx) || self.apu_event;

        let intr = if self.tima_overflowed {
            self.tima = self.tma;
            self.tima_overflowed = false;
            true
        } else {
            false
        };
        if self.tac.enable == 0 {
            return intr;
        }
        if !has_bit_fell(tima_clock_fall_bit(self.tac.clock_select)) {
            return intr;
        }

        // After TIMA overflows into 0, the interrupt and loading TMA to TIMA
        // are delayed by one mcycle.
        if self.tima == 0xFF {
            self.tima_overflowed = true;
            self.tima = 0;
        } else {
            self.tima += 1;
        }

        intr
    }
}

/// Which bit of SYS_CLOCK should fall for TIMA to be incremented.
#[inline]
fn tima_clock_fall_bit(clock_select: u8) -> u32 {
    match clock_select {
        1 => 1,
        2 => 3,
        3 => 5,
        0 => 7,
        _ => unreachable!(),
    }
}
