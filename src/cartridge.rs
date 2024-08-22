mod mbc;

use crate::{info::*, log, macros::match_range, EmuError};

#[derive(Default)]
pub(crate) struct Cartidge {
    pub(crate) is_cgb: bool,
    mbc: mbc::Mbc,

    /// Cartridge ROM fixed size on load.
    rom: Box<[u8]>,
    /// External RAM banks are allocated on demand.
    ram: Vec<u8>,
}

impl Cartidge {
    /// Copy the rom and create a new cartridge.
    pub(crate) fn new(rom: &[u8]) -> Result<Self, EmuError> {
        let is_cgb_rom = matches!(rom[CART_CGB_FLAG], CART_CGB_TOO | CART_CGB_ONLY);
        let mbc = mbc::Mbc::from_rom(rom)?;

        if rom.len() % SIZE_ROM_BANK != 0 {
            log::warn("cartridge: ROM size is not a multiple of 16kiB");
        }

        let mut r = Self {
            is_cgb: is_cgb_rom,
            mbc,
            rom: rom.to_vec().into_boxed_slice(),
            ram: Vec::new(),
        };
        r.alloc_ram(1);

        Ok(r)
    }

    pub(crate) fn read(&self, addr: usize) -> u8 {
        // Some ROM sizes may not be multiples of SIZE_ROM_BANK, in such cases
        // an address might overflow on last ROM bank.
        let safe_read = |addr: usize| {
            if addr < self.rom.len() {
                self.rom[addr]
            } else {
                0xFF
            }
        };

        match_range! { v@addr {
            ADDR_ROM0 => { safe_read(self.mbc.rom0_idx * SIZE_ROM_BANK + v) }
            ADDR_ROM1 => { safe_read(self.mbc.rom1_idx * SIZE_ROM_BANK + v) }
            ADDR_EXT_RAM => {
                if self.mbc.ram_enabled {
                    self.ram[self.get_ram_addr(v)]}
                else {
                    0xFF
                }
            }
            _ => { unreachable!() }
        }}
    }

    pub(crate) fn write(&mut self, addr: usize, val: u8) {
        match_range! { v@addr {
            ADDR_ROM0 => { self.mbc.write(addr, val) }
            ADDR_ROM1 => { self.mbc.write(addr, val) }

            ADDR_EXT_RAM => {
                if self.mbc.ram_enabled {
                    let a = self.get_ram_addr(v);
                    self.ram[a] = val;
                }
            }
            _ => { unreachable!() }
        }}
    }

    /// Allocate RAM if insufficient for a given bank.
    fn alloc_ram(&mut self, bank: usize) {
        // Since RAM sizes can vary for different Cartridges and figuring
        // out how much RAM a cartridge should have in advance is not simple.
        // We just allocate RAM banks on demand if unavailable on bank switch.
        let size = (bank + 1) * SIZE_EXT_RAM;
        if size > self.ram.len() {
            self.ram.resize(size, 0);
        }
    }

    fn get_ram_addr(&self, offset: usize) -> usize {
        self.mbc.ram_idx * SIZE_EXT_RAM + offset
    }
}
