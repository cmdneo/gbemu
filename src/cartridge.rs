use crate::{
    info::*,
    macros::{either, match_range},
    mask_usize, EmulatorErr,
};

const ROM_ADDR_MASK: usize = SIZE_ROM_BANK - 1;
const RAM_ADDR_MASK: usize = SIZE_EXT_RAM_BANK - 1;

pub(crate) struct Cartidge {
    pub(crate) is_cgb: bool,
    pub(crate) title: String,

    kind: MbcType,
    rom: Box<[u8]>,
    ram: Box<[u8]>,
    rom0_off: usize,
    rom1_off: usize,
    ram_off: usize,

    // MBC registers
    ram_enabled: bool,
    bank_mode: bool,
    bank_lo: usize,
    bank_hi: usize,
}

#[derive(Debug)]
enum MbcType {
    None,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
    Mbc6,
    Mbc7,
    Mmm01,
    HuC1,
    HuC3,
}

impl Cartidge {
    /// Copy the rom and create a new cartridge.
    pub(crate) fn new(rom: Vec<u8>) -> Result<Self, EmulatorErr> {
        let is_cgb = matches!(rom[CART_CGB_FLAG], CART_CGB_ONLY);
        if is_cgb {
            return Err(EmulatorErr::NotImplemented);
        }

        let title = rom.get(CART_TITLE).map_or(String::new(), |raw| {
            String::from_utf8_lossy(raw).to_string()
        });

        let kind = cart_mbc_type(rom[CART_TYPE_FLAG])?;
        match kind {
            MbcType::None | MbcType::Mbc1 => (),
            _ => return Err(EmulatorErr::NotImplemented),
        }

        let rom_banks = cart_rom_banks(rom[CART_ROM_FLAG])?;
        if rom_banks * SIZE_ROM_BANK != rom.len() {
            return Err(EmulatorErr::RomSizeMismatch);
        }

        let ram_banks = cart_ram_banks(rom[CART_RAM_FLAG])?;
        let ram = vec![0; SIZE_EXT_RAM_BANK * ram_banks];

        eprintln!("-------------Cartridge-------------");
        eprintln!("Title : {title}");
        eprintln!("Mode  : {}", if is_cgb { "CGB" } else { "DMG" });
        eprintln!("MBC   : {kind:?}");
        eprintln!("RAM   : {} KiB", ram_banks * 8);
        eprintln!("ROM   : {} KiB", rom_banks * 16);
        eprintln!();

        Ok(Self {
            is_cgb,
            title,
            kind,
            rom: rom.into_boxed_slice(),
            ram: ram.into_boxed_slice(),
            rom0_off: 0,
            rom1_off: SIZE_ROM_BANK,
            ram_off: 0,
            ram_enabled: false,
            bank_lo: 1,
            bank_hi: 0,
            bank_mode: false,
        })
    }

    pub(crate) fn read(&self, addr: usize) -> u8 {
        match_range! { a@addr {
            ADDR_EXT_RAM => { self.read_ram((addr & RAM_ADDR_MASK) + self.ram_off) }
            ADDR_ROM0 => { self.read_rom((addr & ROM_ADDR_MASK) + self.rom0_off) }
            ADDR_ROM1 => { self.read_rom((addr & ROM_ADDR_MASK) + self.rom1_off) }
            _ => { 0xFF }
        }}
    }

    pub(crate) fn write(&mut self, addr: usize, val: u8) {
        if ADDR_EXT_RAM.contains(&addr) {
            self.write_ram((addr & RAM_ADDR_MASK) + self.ram_off, val);
            return;
        }

        match self.kind {
            MbcType::None => (),
            MbcType::Mbc1 => self.write_mbc1(addr, val),
            MbcType::Mbc2 => self.write_mbc2(addr, val),
            MbcType::Mbc3 => self.write_mbc3(addr, val),
            MbcType::Mbc5 => self.write_mbc5(addr, val),
            MbcType::Mbc6 => self.write_mbc6(addr, val),
            MbcType::Mbc7 => self.write_mbc7(addr, val),
            MbcType::Mmm01 => self.write_mmm01(addr, val),
            MbcType::HuC1 => self.write_huc1(addr, val),
            MbcType::HuC3 => self.write_huc3(addr, val),
        }
    }

    fn write_mbc1(&mut self, addr: usize, val: u8) {
        let val = val as usize;
        match addr {
            // RAM enable: write 0xA
            0x0000..=0x1FFF => self.ram_enabled = val == 0xA,
            // ROM bank: 5 bits
            0x2000..=0x3FFF => self.bank_lo = val & mask_usize(5),
            // RAM bank or Upper bits of ROM bank: 2 bits
            0x4000..=0x5FFF => self.bank_hi = val & mask_usize(2),
            // Banking mode select: 1 bit
            0x6000..=0x7FFF => self.bank_mode = val & mask_usize(1) == 1,
            _ => (),
        }

        if self.bank_lo == 0 {
            self.bank_lo = 1;
        }

        let bank0 = either!(self.bank_mode, self.bank_hi, 0);
        self.ram_off = bank0 << 13;
        self.rom0_off = bank0 << 19;
        self.rom1_off = self.bank_lo << 14 | self.bank_hi << 19;
    }

    fn write_mbc2(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_mbc3(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_mbc5(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_mbc6(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_mbc7(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_mmm01(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_huc1(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn write_huc3(&mut self, addr: usize, val: u8) {
        _ = (addr, val);
        todo!();
    }

    fn read_rom(&self, addr: usize) -> u8 {
        *self.rom.get(addr).unwrap_or(&0xFF)
    }

    fn read_ram(&self, addr: usize) -> u8 {
        *self.ram.get(addr).unwrap_or(&0xFF)
    }

    fn write_ram(&mut self, addr: usize, val: u8) {
        if let Some(v) = self.ram.get_mut(addr) {
            *v = val;
        }
    }
}

fn cart_mbc_type(v: u8) -> Result<MbcType, EmulatorErr> {
    use MbcType::*;

    Ok(match v {
        0x00 => None,
        0x01 => Mbc1,
        0x02 => Mbc1,
        0x03 => Mbc1,
        0x05 => Mbc2,
        0x06 => Mbc2,
        0x08 => None,
        0x09 => None,
        0x0B => Mmm01,
        0x0C => Mmm01,
        0x0D => Mmm01,
        0x0F => Mbc3,
        0x10 => Mbc3,
        0x11 => Mbc3,
        0x12 => Mbc3,
        0x13 => Mbc3,
        0x19 => Mbc5,
        0x1A => Mbc5,
        0x1B => Mbc5,
        0x1C => Mbc5,
        0x1D => Mbc5,
        0x1E => Mbc5,
        0x20 => Mbc6,
        0x22 => Mbc7,
        0xFE => HuC3,
        0xFF => HuC1,
        _ => return Err(EmulatorErr::UnknownMBC),
    })
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
