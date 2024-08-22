use crate::{
    info::*,
    mem::Mmu,
    regs::{IntData, TimerCtrl},
};

#[derive(Default)]
pub(crate) struct Timer {
    /// M-cycles left before incrementing DIVA.
    div_counter: u32,
    /// M-cycles .
    tima_counter: u32,
    /// Time period for TIMA in M-cycles.
    tima_period: u32,
}

impl Timer {
    pub(crate) fn new() -> Self {
        Self {
            tima_period: u32::MAX,
            ..Default::default()
        }
    }

    pub(crate) fn step(&mut self, mmu: &mut Mmu, mcycles: u32) {
        let div = mmu.read(IO_DIV);
        let tac = TimerCtrl::new(mmu.read(IO_TAC));

        let (ctr, inc_by) = cyclic_add(DIV_MPERIOD, self.div_counter, mcycles);
        let div = div.wrapping_add(inc_by as u8);
        self.div_counter = ctr;
        mmu.set_reg(IO_DIV, div);

        // If TIMA time period changes then reset counter for it.
        let new_period = TMA_MPERIODS[tac.clock_select as usize];
        if new_period != self.tima_period {
            self.tima_period = new_period;
            self.tima_counter = 0;
        }

        if tac.enable == 0 {
            return;
        }

        let tma = mmu.read(IO_TMA);
        let tima = mmu.read(IO_TIMA);
        let (ctr, inc_by) = cyclic_add(self.tima_period, self.tima_counter, mcycles);
        self.tima_counter = ctr;

        // On overflow raise timer interrupt and reset TIMA to TMA.
        if (0xFF - tima as u32) < inc_by {
            let mut iflags = IntData::new(mmu.read(IO_IF));
            iflags.timer = 1;
            mmu.set_reg(IO_TIMA, tma);
            mmu.set_reg(IO_IF, iflags.read());
        } else {
            mmu.set_reg(IO_TIMA, tima + inc_by as u8);
        }
    }
}

/// Cyclic add to `value` modulo `value_max`.
/// Return result and number of times the value wrapped around.
fn cyclic_add(value_max: u32, value: u32, inc_by: u32) -> (u32, u32) {
    if inc_by < value_max - value {
        (value + inc_by, 0)
    } else {
        let left = inc_by - (value_max - value);
        (left % value_max, left / value_max + 1)
    }
}
