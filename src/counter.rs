#[derive(Default, bincode::Encode, bincode::Decode)]
pub(crate) struct Counter {
    ticks: u32,
    period: u32,
}

impl Counter {
    /// Create a counter, a counter with period of 0 never triggers.
    pub(crate) fn new(period: u32) -> Self {
        Self { period, ticks: 0 }
    }

    pub(crate) fn get_period(&self) -> u32 {
        self.period
    }

    /// Tick and return the number of times the counter overflowed/triggered.
    #[inline]
    pub(crate) fn tick(&mut self, elapsed: u32) -> u32 {
        if self.period == 0 {
            return 0;
        }

        // Cyclic subtract `elapsed` from `ticks`.
        if elapsed < self.ticks {
            self.ticks -= elapsed;
            0
        } else {
            let excess = elapsed - self.ticks;
            self.ticks = self.period - excess % self.period;
            excess / self.period + 1
        }
    }
}
