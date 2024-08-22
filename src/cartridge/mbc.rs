use crate::{
    info::{CART_TYPE, SIZE_ROM_BANK},
    EmuError,
};

#[derive(Default)]
pub(crate) struct Mbc {
    /// Type of the Memory Bank Controller present in cartridge,
    /// which needs to be emulated as part of the memory system.
    kind: MbcType,

    /// Current ROM-bank-0, can be zero.
    pub(crate) rom0_idx: usize,
    /// Current ROM-bank-1, lower byte should never be 0x00, change to 0x01.
    pub(crate) rom1_idx: usize,
    /// Current External-RAM-Bank.
    pub(crate) ram_idx: usize,
    /// In some MCBs RAM needs to be enabled before reading/writing.
    pub(crate) ram_enabled: bool,

    bank_reg1: u8,
    bank_reg2: u8,
    bank_mode: u8,
    max_rom_banks: usize,
}

impl Mbc {
    pub(crate) fn from_rom(rom: &[u8]) -> Result<Self, EmuError> {
        let kind = CART_MBC_TYPE_TABLE[rom[CART_TYPE] as usize];

        match kind {
            MbcType::None | MbcType::Mbc1 => (),
            MbcType::Unknown => return Err(EmuError::UnknownMBC),
            _ => unimplemented!(),
        }

        Ok(Self {
            max_rom_banks: rom.len().div_ceil(SIZE_ROM_BANK) + 1,
            kind,
            rom0_idx: 0,
            rom1_idx: 1,
            ram_idx: 0,
            ..Default::default()
        })
    }

    pub(crate) fn write(&mut self, addr: usize, val: u8) {
        match self.kind {
            MbcType::Unknown => panic!("Unknown MBC type found"),
            MbcType::None => (),
            MbcType::Mbc1 => self.mbc1_write(addr, val),

            MbcType::Mbc2 => todo!(),
            MbcType::Mbc3 => todo!(),
            MbcType::Mbc5 => todo!(),
            MbcType::Mbc6 => todo!(),
            MbcType::Mbc7 => todo!(),
            MbcType::Mmm01 => todo!(),
            MbcType::HuC1 => todo!(),
            MbcType::HuC3 => todo!(),
        }

        // For MBC one only
        self.rom1_idx %= self.max_rom_banks;
        if mask_val(self.rom1_idx as u8, 5) == 0 {
            self.rom1_idx |= 0x01;
        }
    }

    // pub(crate) fn get_addr_mbc1(&self, abs_addr: usize) -> usize {
    //     match self.kind {}
    // }

    // Each method xxxx_write handles writes for specified MBCs.
    // The address ranges are the ones which MBCs.

    fn mbc1_write(&mut self, addr: usize, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enabled = val == 0xA,
            0x2000..=0x3FFF => self.bank_reg1 = mask_val(val, 5),
            0x4000..=0x5FFF => self.bank_reg2 = mask_val(val, 2),
            0x6000..=0x7FFF => self.bank_mode = mask_val(val, 1),
            _ => {}
        }

        if self.bank_reg1 == 0 {
            self.bank_reg1 = 1;
        }

        // Calculate addresses as specified by MBC-1.
        let b1 = (self.bank_reg2 << 5) | self.bank_reg1;
        self.rom1_idx = b1 as usize % self.max_rom_banks;

        if self.bank_mode == 0 {
            self.rom0_idx = 0;
            self.ram_idx = 1;
        } else {
            let b0 = self.bank_reg2 << 5;
            self.rom0_idx = b0 as usize % self.max_rom_banks;
            self.ram_idx = self.bank_reg2 as usize;
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
enum MbcType {
    #[default]
    Unknown,
    None,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
    Mbc6,
    Mbc7,
    Mmm01,
    // M161,
    HuC1,
    HuC3,
}

/// MBC type table, indexed by the value of CART_TYPE byte in cartridge header.
const CART_MBC_TYPE_TABLE: [MbcType; 256] = {
    use MbcType::*;
    let mut a = [MbcType::Unknown; 256];

    a[0x00] = None;
    a[0x01] = Mbc1;
    a[0x02] = Mbc1;
    a[0x03] = Mbc1;
    a[0x05] = Mbc2;
    a[0x06] = Mbc2;
    a[0x08] = None;
    a[0x09] = None;
    a[0x0B] = Mmm01;
    a[0x0C] = Mmm01;
    a[0x0D] = Mmm01;
    a[0x0F] = Mbc3;
    a[0x10] = Mbc3;
    a[0x11] = Mbc3;
    a[0x12] = Mbc3;
    a[0x13] = Mbc3;
    a[0x19] = Mbc5;
    a[0x1A] = Mbc5;
    a[0x1B] = Mbc5;
    a[0x1C] = Mbc5;
    a[0x1D] = Mbc5;
    a[0x1E] = Mbc5;
    a[0x20] = Mbc6;
    a[0x22] = Mbc7;
    a[0xFE] = HuC3;
    a[0xFF] = HuC1;
    a
};

#[inline(always)]
fn mask_val(val: u8, bit_cnt: u8) -> u8 {
    val & !(!0 << bit_cnt)
}
