use bincode::{Decode, Encode};

use crate::{counter::Counter, info};

const DAYS_MAX: u32 = 0x1FF; // 9-bits are used for day

#[derive(Default, Encode, Decode)]
pub(crate) struct Mbc3Rtc {
    counter: Counter,
    latched: Option<[u8; 5]>,
    clk: Clock,

    halt: bool,
    overflowed: bool,
}

#[derive(Default, Encode, Decode)]
struct Clock {
    s: u32,
    m: u32,
    h: u32,
    d: u32,
}

impl Clock {
    fn tick(&mut self, seconds: u32) {
        let mut x = seconds;
        (self.s, x) = mod_add(self.s, x, 60);
        (self.m, x) = mod_add(self.m, x, 60);
        (self.h, x) = mod_add(self.h, x, 24);
        self.d = self.d.saturating_add(x);
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
        if self.halt {
            return;
        }

        self.clk.tick(self.counter.tick(dots));
        self.overflowed = self.clk.d > DAYS_MAX;
    }

    pub(crate) fn set_latching(&mut self, enable: bool) {
        if enable {
            self.latched = Some([
                self.clk.s as u8,
                self.clk.m as u8,
                self.clk.h as u8,
                self.clk.d as u8,
                self.read_reg_c() as u8,
            ]);
        } else {
            self.latched = None;
        }
    }

    pub(crate) fn read(&self, reg: usize) -> u8 {
        if let Some(saved) = self.latched {
            return *saved.get(reg - 0x8).unwrap_or(&0xFF);
        }

        (match reg {
            0x8 => self.clk.s,
            0x9 => self.clk.m,
            0xA => self.clk.h,
            0xB => self.clk.d,
            0xC => self.read_reg_c(),
            _ => 0xFF,
        }) as u8
    }

    pub(crate) fn write(&mut self, reg: usize, val: u8) {
        let val = val as u32;
        match reg {
            0x8 => self.clk.s = val & mask(6),
            0x9 => self.clk.m = val & mask(6),
            0xA => self.clk.h = val & mask(5),
            0xB => self.clk.d = val,
            0xC => self.write_reg_c(val & (1 | 0b11 << 6)),
            _ => (),
        }
    }

    fn read_reg_c(&self) -> u32 {
        // MBC3 RTC 0xC register:
        // Bit 0: Day 8th bit, Bit 6: Halt, Bit 7: Overflow
        ((self.clk.d >> 8) & 1) | (self.halt as u32) << 6 | (self.overflowed as u32) << 7
    }

    fn write_reg_c(&mut self, val: u32) {
        if val & 1 == 1 {
            self.clk.d |= 1 << 8;
        } else {
            self.clk.d &= !(1 << 8);
        }

        self.halt = (val >> 6) & 1 == 1;
        self.overflowed = (val >> 7) & 1 == 1;
    }
}

const fn mod_add(v: u32, u: u32, modulo: u32) -> (u32, u32) {
    ((v + u) % modulo, (v + u) / modulo)
}

#[inline(always)]
const fn mask(bits: u32) -> u32 {
    if bits == u32::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}
