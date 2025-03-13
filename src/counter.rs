#[derive(Default)]
pub(crate) struct Counter {
    ticks: u32,
    period: u32,
}

impl Counter {
    pub(crate) fn new(period: u32) -> Self {
        Self { period, ticks: 0 }
    }

    pub(crate) fn get_period(&self) -> u32 {
        return self.period;
    }

    #[inline]
    pub(crate) fn tick(&mut self, elapsed: u32) -> u32 {
        if self.period == 0 {
            return 0;
        }

        // Cyclic add to `ticks` modulo `period`.
        if elapsed < self.period - self.ticks {
            self.ticks += elapsed;
            0
        } else {
            let excess = elapsed - (self.period - self.ticks);
            self.ticks = excess % self.period;
            excess / self.period + 1
        }
    }
}
