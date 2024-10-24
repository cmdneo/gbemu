use crate::regs::SerialCtrl;

#[derive(Default)]
pub(crate) struct Serial {
    pub(crate) is_2x: bool,

    // Registers owned by it
    pub(crate) sc: SerialCtrl,
    pub(crate) sb: u8,

    // M-cycles counter, incement after reaches period.
    counter: u16,
    period: u16,
    bits_done: u16,
    transferring: bool,
}

impl Serial {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn tick(&mut self, mcycles: u16, is_cgb_cart: bool) -> bool {
        if self.sc.tx_enable == 0 {
            return false;
        }

        // Start a new transfer if enabled from the next cycle.
        if !self.transferring {
            // There is no external clock as this is an emulator, use a default.
            self.period = if self.sc.clock_select == 0 {
                1
            } else {
                get_mperiod(self.sc.clock_speed, is_cgb_cart, self.is_2x)
            };
            self.bits_done = 0;
            self.counter = 0;
            self.transferring = true;
            return false;
        }

        let (ctr, inc_by) = cyclic_add(self.period, self.counter, mcycles);
        self.counter = ctr;
        self.bits_done += inc_by;
        // Outgoing bits are shifted out.
        self.sb = self.sb.wrapping_shl(inc_by as u32);

        if self.bits_done < 8 {
            return false;
        }

        // println!("Serial transfer done");
        // Transfer complete
        self.transferring = false;
        self.sc.tx_enable = 0;
        true
    }
}

/// Get period for each cycle in M-cycles for serial transfer.
fn get_mperiod(clock_speed: u8, is_cgb_cart: bool, is_2x: bool) -> u16 {
    if !is_cgb_cart {
        128
    } else {
        match (clock_speed == 1, is_2x) {
            (true, true) => 4,
            (true, false) => 8,
            (false, true) => 256,
            (false, false) => 128,
        }
    }
}

/// Cyclic add to `value` modulo `value_max`.
/// Return result and number of times the value wrapped around.
fn cyclic_add(max_val: u16, val: u16, inc_by: u16) -> (u16, u16) {
    if inc_by < max_val - val {
        (val + inc_by, 0)
    } else {
        let left = inc_by - (max_val - val);
        (left % max_val, left / max_val + 1)
    }
}
