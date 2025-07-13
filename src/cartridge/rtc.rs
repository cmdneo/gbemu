use crate::{counter::Counter, info, macros::bit_fields};

#[derive(Default, bincode::Encode, bincode::Decode)]
pub(crate) struct Mbc3Rtc {
    counter: Counter,
    latched: Option<[u8; 5]>,

    sec: u8,
    min: u8,
    hr: u8,
    day: u8,
    #[bincode(with_serde)]
    ctrl: RtcCtrlReg,
}

bit_fields! {
    // MBC3 RTC 0xC register:
    #[derive(serde::Serialize, serde::Deserialize)]
    struct RtcCtrlReg<u8> {
        day: 1,
        _0: 5,
        halt: 1,
        overflow: 1,
    }
}

impl Mbc3Rtc {
    pub(crate) fn new() -> Self {
        Self {
            counter: Counter::new(info::FREQUENCY),
            ..Default::default()
        }
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        if self.ctrl.halt == 1 {
            return;
        }

        for _ in 0..self.counter.tick(dots) {
            self.adjust_registers(true);
        }
    }

    pub(crate) fn set_latching(&mut self, enable: bool) {
        if enable {
            self.latched = Some([self.sec, self.min, self.hr, self.day, self.ctrl.read()]);
        } else {
            self.latched = None;
        }
    }

    pub(crate) fn read(&self, reg_id: usize) -> u8 {
        if let Some(saved) = self.latched {
            return *saved.get(reg_id - 0x8).unwrap_or(&0xFF);
        }

        match reg_id {
            0x8 => self.sec,
            0x9 => self.min,
            0xA => self.hr,
            0xB => self.day,
            0xC => self.ctrl.read(),
            _ => 0xFF,
        }
    }

    pub(crate) fn write(&mut self, reg_id: usize, val: u8) {
        match reg_id {
            0x8 => self.sec = val,
            0x9 => self.min = val,
            0xA => self.hr = val,
            0xB => self.day = val,
            0xC => self.ctrl.write(val & 0xC1),
            _ => (),
        }

        self.adjust_registers(false);
    }

    fn adjust_registers(&mut self, inc: bool) {
        let mut rst = inc;
        (self.sec, rst) = adjust_reg(self.sec, 59, 6, rst);
        (self.min, rst) = adjust_reg(self.min, 59, 6, rst);
        (self.hr, rst) = adjust_reg(self.hr, 23, 5, rst);
        (self.day, rst) = adjust_reg(self.day, 255, 8, rst);
        (self.ctrl.day, rst) = adjust_reg(self.ctrl.day, 1, 1, rst);
        self.ctrl.overflow |= rst as u8; // Is only reset when written to
    }
}

/// Correct RTC register value, it involves masking out extraneous
/// bits and resetting the register if increment on `wrap_on` value.
/// Note that increment on a value higher that wrap_on or register's max
/// value does not count as a reset.
/// Returns new value and true if register reset.
fn adjust_reg(old: u8, wrap_on: u8, width: u32, inc: bool) -> (u8, bool) {
    if inc {
        if old == wrap_on {
            (0, true)
        } else {
            ((old.wrapping_add(1)) & mask(width), false)
        }
    } else {
        (old & mask(width), false)
    }
}

#[inline(always)]
const fn mask(bits: u32) -> u8 {
    if bits == u8::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}
