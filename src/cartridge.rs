mod mbc;
mod rtc;

use bincode::{Decode, Encode};

use crate::{info::*, macros::match_range, EmulatorErr};
use mbc::{Mbc, MbcKind};

#[derive(Encode, Decode)]
pub(crate) struct Cartidge {
    pub(crate) is_cgb: bool,
    pub(crate) title: String,
    pub(crate) rom: Box<[u8]>,
    pub(crate) ram: Box<[u8]>,
    mbc: Mbc,
}

const MBC2_BUILTIN_RAM_SIZE: usize = 512;

impl Cartidge {
    /// Copy the rom and create a new cartridge.
    pub(crate) fn new(rom: Vec<u8>) -> Result<Self, EmulatorErr> {
        if rom.len() % SIZE_ROM_BANK != 0 {
            return Err(EmulatorErr::InvalidRomSize);
        }

        let is_cgb = matches!(rom[CART_CGB_FLAG], CART_CGB_ONLY);
        let mbc = Mbc::new(rom[CART_TYPE_FLAG])?;
        let title = rom.get(CART_TITLE).map_or(String::new(), |raw| {
            let mut tmp = String::from_utf8_lossy(raw).to_string();
            tmp.retain(|c| c.is_ascii_graphic() || c == ' ');
            tmp
        });

        let rom_banks = cart_rom_banks(rom[CART_ROM_FLAG])?;
        let ram_banks = cart_ram_banks(rom[CART_RAM_FLAG])?;
        let ram = vec![
            0;
            if matches!(mbc.kind, MbcKind::Mbc2 { .. }) {
                MBC2_BUILTIN_RAM_SIZE
            } else {
                SIZE_EXT_RAM_BANK * ram_banks
            }
        ];

        eprintln!("-------------Cartridge-------------");
        eprintln!("Title : {title}");
        eprintln!("Mode  : {}", if is_cgb { "CGB" } else { "DMG" });
        eprintln!("MBC   : {}", mbc.kind.name());
        eprintln!("RAM   : {} KiB", ram_banks * 8);
        eprintln!("ROM   : {} KiB", rom_banks * 16);
        eprintln!();

        // Return error after printing cartridge info, useful for debugging.
        if is_cgb {
            return Err(EmulatorErr::NotImplemented);
        }
        if rom_banks * SIZE_ROM_BANK != rom.len() {
            return Err(EmulatorErr::RomSizeMismatch);
        }

        Ok(Self {
            is_cgb,
            title,
            rom: rom.into_boxed_slice(),
            ram: ram.into_boxed_slice(),
            mbc,
        })
    }

    pub(crate) fn tick(&mut self, dots: u32) {
        self.mbc.rtc.tick(dots);
    }

    pub(crate) fn read(&self, addr: usize) -> u8 {
        match_range! { a@addr {
            ADDR_EXT_RAM => { self.read_ram(self.mbc.ram_addr(addr)) }
            ADDR_ROM0 => { self.read_rom(self.mbc.rom0_addr(addr)) }
            ADDR_ROM1 => { self.read_rom(self.mbc.rom1_addr(addr)) }
            _ => { 0xFF }
        }}
    }

    pub(crate) fn write(&mut self, addr: usize, val: u8) {
        if ADDR_EXT_RAM.contains(&addr) {
            if self.mbc.ram_enabled {
                self.write_ram(self.mbc.ram_addr(addr), val);
            }
        } else {
            self.mbc.write(addr, val);
        }
    }

    fn read_rom(&self, addr: usize) -> u8 {
        *self.rom.get(addr).unwrap_or(&0xFF)
    }

    fn read_ram(&self, addr: usize) -> u8 {
        if let Some(reg) = self.mbc.kind.get_mbc3_rtc_reg_if_set() {
            self.mbc.rtc.read(reg)
        } else {
            *self.ram.get(addr).unwrap_or(&0xFF)
        }
    }

    fn write_ram(&mut self, addr: usize, val: u8) {
        if let Some(reg) = self.mbc.kind.get_mbc3_rtc_reg_if_set() {
            self.mbc.rtc.write(reg, val);
        } else if let Some(v) = self.ram.get_mut(addr) {
            *v = val;
        }
    }
}

/// Number of ROM banks, each of 16KiB.
fn cart_rom_banks(v: u8) -> Result<usize, EmulatorErr> {
    if v <= 8 {
        Ok(2 << (v as usize))
    } else {
        Err(EmulatorErr::UnknownRomSize)
    }
}

/// Number of RAM banks, each of 8KiB.
fn cart_ram_banks(v: u8) -> Result<usize, EmulatorErr> {
    match v {
        0 => Ok(0),
        1 => Err(EmulatorErr::UnknownRamSize), // Use of this value is unspecified.
        2 => Ok(1),
        3 => Ok(4),
        4 => Ok(16),
        5 => Ok(8),
        _ => Err(EmulatorErr::UnknownRamSize),
    }
}
