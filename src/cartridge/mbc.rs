use bincode::{Decode, Encode};

use super::rtc::Mbc3Rtc;
use crate::{info, EmulatorErr};

#[derive(Encode, Decode)]
pub(crate) struct Mbc {
    pub(crate) kind: MbcKind,
    pub(crate) ram_enabled: bool,
    pub(crate) rtc: Mbc3Rtc,

    ram_mask: usize,
    rom_mask: usize,
    ram_bank: usize,
    rom0_bank: usize,
    rom1_bank: usize,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub(crate) enum MbcKind {
    None,
    Mbc1 {
        rom_bank_lo: usize,
        rom_bank_hi: usize,
        bank_mode: bool,
    },
    Mbc2 {
        rom_bank: usize,
    },
    Mbc3 {
        rom_bank: usize,
        ram_rtc_bank: usize,
    },
    Mbc5 {
        rom_bank_lo: usize,
        rom_bank_hi: usize,
        ram_bank: usize,
        has_rumble: bool,
    },
    Mbc6,
    Mbc7,
    Mmm01,
    HuC1,
    HuC3,
}

impl MbcKind {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            MbcKind::None => "None",
            MbcKind::Mbc1 { .. } => "MBC1",
            MbcKind::Mbc2 { .. } => "MBC2",
            MbcKind::Mbc3 { .. } => "MBC3",
            MbcKind::Mbc5 { .. } => "MBC5",
            MbcKind::Mbc6 => "MBC6",
            MbcKind::Mbc7 => "MBC7",
            MbcKind::Mmm01 => "MMM01",
            MbcKind::HuC1 => "HuC1",
            MbcKind::HuC3 => "HuC3",
        }
    }

    pub(crate) fn get_mbc3_rtc_reg_if_set(&self) -> Option<usize> {
        match self {
            Self::Mbc3 {
                ram_rtc_bank: addr @ 0x8..=0xC,
                ..
            } => Some(*addr),
            _ => None,
        }
    }
}

impl Mbc {
    pub(crate) fn new(mbc_id: u8) -> Result<Self, EmulatorErr> {
        let kind = match mbc_id {
            0x00 => MbcKind::None,
            0x01..=0x03 => MbcKind::Mbc1 {
                rom_bank_lo: 1,
                rom_bank_hi: 0,
                bank_mode: false,
            },
            0x05 | 0x06 => MbcKind::Mbc2 { rom_bank: 1 },
            0x08..=0x09 => MbcKind::None,
            0x0B..=0x0D => MbcKind::Mmm01,
            0x0F..=0x13 => MbcKind::Mbc3 {
                rom_bank: 1,
                ram_rtc_bank: 0,
            },
            0x19..=0x1E => MbcKind::Mbc5 {
                rom_bank_lo: 0,
                rom_bank_hi: 0,
                ram_bank: 0,
                has_rumble: matches!(mbc_id, 0x1C..=0x1E),
            },
            0x20 => return Err(EmulatorErr::NotImplemented), // MbcKind::Mbc6,
            0x22 => return Err(EmulatorErr::NotImplemented), // MbcKind::Mbc7,
            0xFE => return Err(EmulatorErr::NotImplemented), // MbcKind::HuC3,
            0xFF => return Err(EmulatorErr::NotImplemented), // MbcKind::HuC1,
            _ => return Err(EmulatorErr::UnknownMBC),
        };
        let (rom_mask, ram_mask) = get_rom_ram_addr_mask(kind);

        Ok(Self {
            kind,
            ram_enabled: false,
            rtc: Mbc3Rtc::new(),
            ram_mask,
            rom_mask,
            ram_bank: 0,
            rom0_bank: 0,
            rom1_bank: 1,
        })
    }

    pub(crate) fn write(&mut self, addr: usize, v: u8) {
        let v = v as usize;
        let is_0xa = v & mask(4) == 0xA;

        // In some cartridges if ROM bank is written 0 it is translated to 1.
        let fix_bank_num = |b: &mut usize| {
            if *b == 0 {
                *b = 1;
            }
        };

        (self.ram_bank, self.rom0_bank, self.rom1_bank) = match &mut self.kind {
            MbcKind::None => (0, 0, 0),

            MbcKind::Mbc1 {
                rom_bank_lo,
                rom_bank_hi,
                bank_mode,
            } => {
                match addr {
                    // RAM enable
                    0x0000..=0x1FFF => self.ram_enabled = is_0xa,
                    // ROM bank
                    0x2000..=0x3FFF => *rom_bank_lo = v & mask(5),
                    // RAM bank or Upper bits of ROM bank
                    0x4000..=0x5FFF => *rom_bank_hi = v & mask(2),
                    // Banking mode
                    0x6000..=0x7FFF => *bank_mode = v & 1 == 1,
                    _ => (),
                }

                // In MBC1 rom_bank_hi acts as both: RAM bank number and
                // upper bits of ROM bank(0 & 1) number as per bank_mode.
                fix_bank_num(rom_bank_lo);
                let bank0 = if *bank_mode { *rom_bank_hi } else { 0 };
                (bank0, bank0 << 5, *rom_bank_lo | *rom_bank_hi << 5)
            }

            MbcKind::Mbc2 { rom_bank } => {
                match addr {
                    // RAM enable
                    0x0000..=0x00FF => self.ram_enabled = is_0xa,
                    // ROM bank
                    0x0100..=0x3FFF => *rom_bank = v & mask(4),
                    _ => (),
                }

                fix_bank_num(rom_bank);
                (0, 0, *rom_bank)
            }

            MbcKind::Mbc3 {
                rom_bank,
                ram_rtc_bank,
            } => {
                match addr {
                    // RAM & Timer enable
                    0x0000..=0x1FFF => self.ram_enabled = is_0xa,
                    // ROM bank
                    0x2000..=0x3FFF => *rom_bank = v & mask(7),
                    // RAM bank or RTC register
                    0x4000..=0x5FFF => *ram_rtc_bank = v & mask(4),
                    // Latch clock data, writing first 0x0 then 0x1 latches the clock data.
                    0x6000..=0x7FFF => match v {
                        0 => self.rtc.set_latching(false),
                        1 => self.rtc.set_latching(true),
                        _ => (),
                    },
                    _ => (),
                }

                fix_bank_num(rom_bank);
                (*ram_rtc_bank, 0, *rom_bank)
            }

            MbcKind::Mbc5 {
                rom_bank_lo,
                rom_bank_hi,
                ram_bank,
                has_rumble,
            } => {
                match addr {
                    // RAM enable
                    0x0000..=0x1FFF => self.ram_enabled = is_0xa,
                    // ROM bank low
                    0x2000..=0x2FFF => *rom_bank_lo = v,
                    // ROM bank high
                    0x3000..=0x3FFF => *rom_bank_hi = v & 1,
                    // RAM bank
                    0x4000..=0x5FFF => *ram_bank = v & mask(if *has_rumble { 3 } else { 4 }),
                    _ => (),
                }

                (*ram_bank, 0, *rom_bank_lo | *rom_bank_hi << 8)
            }

            MbcKind::Mbc6 => unimplemented!(),
            MbcKind::Mbc7 => unimplemented!(),
            MbcKind::Mmm01 => unimplemented!(),
            MbcKind::HuC1 => unimplemented!(),
            MbcKind::HuC3 => unimplemented!(),
        };
    }

    #[inline]
    pub(crate) fn ram_addr(&self, addr: usize) -> usize {
        (addr & self.ram_mask) | (self.ram_bank * info::SIZE_EXT_RAM_BANK)
    }

    #[inline]
    pub(crate) fn rom0_addr(&self, addr: usize) -> usize {
        (addr & self.rom_mask) | (self.rom0_bank * info::SIZE_ROM_BANK)
    }

    #[inline]
    pub(crate) fn rom1_addr(&self, addr: usize) -> usize {
        (addr & self.rom_mask) | (self.rom1_bank * info::SIZE_ROM_BANK)
    }
}

#[inline(always)]
const fn mask(bits: u32) -> usize {
    if bits == usize::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}

fn get_rom_ram_addr_mask(mbc: MbcKind) -> (usize, usize) {
    match mbc {
        MbcKind::Mbc2 { .. } => (info::SIZE_ROM_BANK - 1, mask(9)),
        _ => (info::SIZE_ROM_BANK - 1, info::SIZE_EXT_RAM_BANK - 1),
    }
}
