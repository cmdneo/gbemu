use std::io::Write;

use crate::{counter::Counter, regs::SerialCtrl};

#[derive(Default, bincode::Encode, bincode::Decode)]
pub(crate) struct Serial {
    pub(crate) debug_serial: bool,
    pub(crate) is_2x: bool,
    is_cgb: bool,

    // Registers owned by it
    #[bincode(with_serde)]
    pub(crate) sc: SerialCtrl,
    pub(crate) sb: u8,

    counter: Counter,
    bits_done: u32,
    transferring: bool,
}

impl Serial {
    pub(crate) fn new(is_cgb: bool) -> Self {
        Self {
            is_cgb,
            ..Default::default()
        }
    }

    /// Tick and return true if SERIAL interrupt has been requested.
    pub(crate) fn tick(&mut self, mcycles: u32) -> bool {
        if self.sc.tx_enable == 0 {
            return false;
        }

        // Start a new transfer from the next cycle.
        if !self.transferring {
            if self.debug_serial {
                print!("{}", self.sb as char);
                std::io::stdout().flush().unwrap();
            }

            self.counter = Counter::new(get_period_in_mcycles(self.sc, self.is_cgb, self.is_2x));
            self.bits_done = 0;
            self.transferring = true;
            return false;
        }

        if self.counter.get_period() == 0 {
            return false;
        }

        let inc_by = self.counter.tick(mcycles);
        self.bits_done += inc_by;

        if self.bits_done < 8 {
            return false;
        }

        // Transfer complete. Indicate a disconnected link by setting IN=0xFF.
        self.sb = 0xFF;
        self.transferring = false;
        self.sc.tx_enable = 0;
        true
    }
}

/// Get period for 1 serial-cycle(transfers 1-bit).
fn get_period_in_mcycles(sc: SerialCtrl, is_cgb: bool, is_2x: bool) -> u32 {
    if sc.clock_select == 0 {
        return 0; // External clock is absent.
    }

    if !is_cgb {
        128
    } else {
        match (sc.clock_speed == 1, is_2x) {
            (true, true) => 4,
            (true, false) => 8,
            (false, true) => 256,
            (false, false) => 128,
        }
    }
}
