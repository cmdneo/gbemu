use crate::{
    cartridge::Cartidge,
    info::*,
    macros::{in_ranges, match_range},
    regs::{ActionButtons, CgbPaletteIndex, DPad, IntData, JoyPad, LcdStat},
};

/// The memory sub-system, contains the `Cartridge`, `IoRegisters` and
/// all global configuration information.
// Why put all config info in it? Because it is used by both CPU and PPU,
// so this is a good place put shared information needed by both.
pub(crate) struct Mmu {
    /// Is operating in Color mode, aka dual-speed mode.
    pub(crate) is_cgb: bool,
    pub(crate) cart: Cartidge,

    // Non-cartridge memory regions.
    /// VRAM, has two banks in CGB mode.
    pub(crate) vram: [[u8; SIZE_VRAM_BANK]; VRAM_BANKS],
    wram: [[u8; SIZE_WRAM_BANK]; WRAM_BANKS],
    /// Object attribute memory, each attribute is of 4 bytes.
    pub(crate) oam: [u8; SIZE_OAM],
    regs: [u8; SIZE_IO_REGS],
    hram: [u8; SIZE_HRAM],
    ie: u8,

    // CGB color palettes are stored in a seperate RAM accesed indirectly.
    pub(crate) bg_palette: [u8; SIZE_CGB_PALETTE],
    pub(crate) obj_palette: [u8; SIZE_CGB_PALETTE],

    // Joypad state.
    dpad: DPad,
    buttons: ActionButtons,
    // OAM-DMA state keeping.
    oam_dma: Option<OamDma>,
}

#[derive(Clone, Copy)]
struct OamDma {
    src: usize,
    copied: usize,
    count: usize,
}

impl Mmu {
    pub(crate) fn new(cartd: Cartidge) -> Self {
        let mut r = Self {
            cart: cartd,
            ..Default::default()
        };
        r.set_reg(IO_SVBK, 1); // WRAM-1 bank is never 0.
        r
    }

    /// Perform if any DMA is pending step-by-step.
    pub(crate) fn step(&mut self, mcycles: u32) {
        let mut dma = if let Some(d) = self.oam_dma {
            d
        } else {
            return;
        };

        for _ in 0..mcycles {
            if dma.copied == dma.count {
                break;
            }

            self.oam[dma.copied] = self.read(dma.src + dma.copied);
            dma.copied += 1;
        }

        if dma.copied == dma.count {
            self.oam_dma = None;
        } else {
            self.oam_dma = Some(dma);
        }
    }

    // On real hardware some memory locations are not inaccessible for reading
    // or writing or both because of some PPU mode or it is a register which
    // does not support either read or write.
    // In our emulator we do check for such conditions when writing data, but
    // not when reading as reading does not have any side-effects.

    /// Reads one byte, use when executing instructions by CPU.
    pub(crate) fn read_cpu(&self, addr: u16) -> u8 {
        self.read(addr as usize)
    }

    /// Writes one byte, use when executing instructions by CPU.
    /// Writes to read-only registers are ignored, use `reg_set` for that.
    pub(crate) fn write_cpu(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        let (vidx, widx) = self.get_vram_wram_idx();
        debug_assert!(widx != 0);

        if !self.is_accessible(addr) {
            return;
        }
        if is_cart_addr(addr) {
            self.cart.write(addr, val);
            return;
        }

        let mode = self.get_mode();
        // Ignore writes to graphics related memory regions which are
        // inaccessible during certain PPU modes.
        match_range! { a@addr {
            ADDR_VRAM => {
                if mode != MODE_DRAW {
                    self.vram[vidx][a] = val
                }
            }
            ADDR_WRAM0 => { self.wram[0][a] = val}
            ADDR_WRAM1 => { self.wram[widx][a] = val }
            ADDR_ECHO_RAM => { self.write_cpu(get_echo_ram_addr(a) as u16, val) }

            ADDR_OAM => {
                if !matches!(mode, MODE_DRAW | MODE_SCAN) {
                    self.oam[a] = val
                }
            }

            ADDR_UNUSABLE => {}
            ADDR_IO_REGS => { self.cpu_reg_write(addr, val) }
            ADDR_HRAM => { self.hram[a] = val}
            ADDR_IE => { self.ie = val }

            _ => { unreachable!() }
        }}
    }

    /// Read data.
    pub(crate) fn read(&self, addr: usize) -> u8 {
        let (vidx, widx) = self.get_vram_wram_idx();
        debug_assert!(widx != 0);

        if is_cart_addr(addr) {
            return self.cart.read(addr);
        }

        match_range! { a@addr {
            ADDR_VRAM => { self.vram[vidx][a] }
            ADDR_WRAM0 => { self.wram[0][a] }
            ADDR_WRAM1 => { self.wram[widx][a] }
            ADDR_ECHO_RAM => { self.read_cpu(get_echo_ram_addr(a) as u16) }
            ADDR_OAM => { self.oam[a] }
            ADDR_UNUSABLE => { 0 }
            ADDR_IO_REGS => {
                match addr {
                    IO_BGPD | IO_OBPD => self.get_cgb_palette_data(addr),
                    _ => self.regs[a],
                }
            }
            ADDR_HRAM => { self.hram[a] }
            ADDR_IE => { self.ie }

            _ => { unreachable!() }
        }}
    }

    /// Get PPU mode.
    pub(crate) fn get_mode(&self) -> u8 {
        LcdStat::new(self.read(IO_STAT)).ppu_mode
    }

    /// Write any IO-register memory location.
    /// Use it for setting register values as some registers are read only.
    pub(crate) fn set_reg(&mut self, addr: usize, val: u8) {
        match_range! { a@addr {
            ADDR_IO_REGS => { self.regs[a] = val }
            ADDR_IE => { self.ie = val }

            _ => { panic!("'{addr:#X}' is not a register addres") }
        }}
    }

    /// Update joypad buttons and Joypad/P1 register.
    /// Also, raise Joypad interrupt condition is met.
    pub(crate) fn update_joypad(&mut self, dpad: DPad, btns: ActionButtons) {
        let jp = JoyPad::new(self.read(IO_JOYPAD));

        let mut nibble = 0u8;
        // In case both buttons classes are selected, then, a button press
        // belonging to any class wil affect its corresponding bit.
        if jp.select_buttons == 0 {
            nibble |= btns.read();
        }
        if jp.select_dpad == 0 {
            nibble |= dpad.read();
        }

        // Flip before writing the lower nibble as for Joypad 0 means pressed.
        nibble = !nibble & mask(4);
        self.set_reg(IO_JOYPAD, jp.read() | nibble);

        // Joypad Interupt occurs when any of 0-3 bits go from high to low.
        for i in 0..4 {
            if (jp.read() >> i) & 1 == 1 && (nibble >> i) & 1 == 0 {
                let mut iflag = IntData::new(self.read(IO_IF));
                iflag.joypad = 1;
                self.set_reg(IO_IF, iflag.read());
                break;
            }
        }

        self.dpad = dpad;
        self.buttons = btns;
    }

    /// Get `IF & IE` as `IntData`.
    pub(crate) fn get_queued_ints(&self) -> IntData {
        let iflag = self.read(IO_IF);
        let ien = self.read(IO_IE);
        IntData::new(iflag & ien)
    }

    /// Writes to a register(or its fields) if it is writable and
    /// performs necessary action corresponding to the register if any.
    ///
    /// Writes to read-only registers(or register fields) are ignored.
    fn cpu_reg_write(&mut self, addr: usize, val: u8) {
        // Get relative address for regs array.
        let a = match_range! { v@addr {
            ADDR_IO_REGS => { v }
            ADDR_IE => {
                self.ie = val;
                return;
            }
            _ => { panic!("{addr:#X} is not a register addres") }
        }};

        // Set value but keep masked bits preserved.
        macro_rules! set {
            ($val:expr, $keep_mask:expr) => {
                self.regs[a] = combine(self.regs[a], $val, $keep_mask)
            };
        }

        match addr {
            IO_BGPD | IO_OBPD => {
                // CGB paletes are locked during when PPU is drawing(Mode-3).
                if self.get_mode() != MODE_DRAW {
                    self.set_cgb_palette_data(addr);
                }
            }

            // Read only registers
            IO_LY | IO_PCM12 | IO_PCM34 => (),

            // Partially writable registers. RO: Read only.
            IO_NR52 => set!(val, mask(4)),  // CHx on? are RO
            IO_STAT => set!(val, mask(3)),  // PPU mode and LYC==LC is RO
            IO_KEY1 => set!(val, !mask(1)), // Only first bit is writable
            IO_RP => set!(val, 1 << 1),     // Second bit[recv] is RO
            IO_JOYPAD => {
                // Update joypad register as per select_dpad/buttons selection.
                set!(val, mask(4)); // Lower 4-bits are RO
                self.update_joypad(self.dpad, self.buttons);
            }

            // Do not write to unused bits of IE and IF, keep them zero.
            IO_IE | IO_IF => set!(val, !mask(5)),

            // Register which when writtten to cause an action.
            IO_DMA => self.start_dma(val),
            IO_DIV => self.regs[a] = 0,
            IO_TMA => {
                // TODO Does setting TMA also reset TIMA?
                self.regs[a] = val;
                self.set_reg(IO_TIMA, val);
            }

            // Available only in CGB mode, otherwise ignore writes.
            IO_VBK | IO_SVBK => {
                // WRAM1 bank number cannot be 0, change it to 1 if 0.
                let val = if addr == IO_SVBK && val == 0 { 1 } else { val };
                if self.is_cgb {
                    self.regs[a] = val;
                }
            }

            // Rest of the registers.
            _ => self.regs[a] = val,
        }
    }

    fn start_dma(&mut self, addr: u8) {
        // DMA address specifies the high-byte value of the 16-bit
        // source address. Valid values for it are from 0x00 to 0xDF.
        // If it is overflowing we just wrap around it.
        let src = ((addr as usize) % 0xDF) << 4;

        // Src is from $XX00 to $XX9F.
        self.oam_dma = Some(OamDma {
            src,
            copied: 0,
            count: ADDR_OAM.count(),
        });

        self.set_reg(IO_DMA, addr);
    }

    // CGB palette management methods
    //---------------------------------------------------------------
    /// Get CGB palette data byte for address set in BGPI/OBPI.
    fn get_cgb_palette_data(&self, pal_data_reg: usize) -> u8 {
        let (addr, _) = self.get_cgb_palette_index(pal_data_reg);

        let a = addr.addr as usize;
        match pal_data_reg {
            IO_BGPD => self.bg_palette[a],
            IO_OBPD => self.obj_palette[a],
            _ => unreachable!(),
        }
    }

    /// Set CGB palette data and increment address if `auto_inc` is 1.
    fn set_cgb_palette_data(&mut self, pal_reg: usize) {
        let (mut addr, addr_reg) = self.get_cgb_palette_index(pal_reg);

        let a = addr.addr as usize;
        match pal_reg {
            IO_BGPD => self.bg_palette[a] = self.read(IO_BGPD),
            IO_OBPD => self.obj_palette[a] = self.read(IO_OBPD),
            _ => unreachable!(),
        }

        if addr.auto_inc == 1 {
            addr.addr += 1;
            self.set_reg(addr_reg, addr.read());
        }
    }

    /// Get CGB BG/OBJ palette index register and its address.
    fn get_cgb_palette_index(&self, pal_data_reg: usize) -> (CgbPaletteIndex, usize) {
        let addr_reg = match pal_data_reg {
            IO_OBPD => IO_OBPI,
            IO_BGPD => IO_BGPI,
            _ => panic!("{pal_data_reg:#X} is not a CGB palette data register"),
        };

        (CgbPaletteIndex::new(self.read(addr_reg)), addr_reg)
    }

    // Utility methods
    //---------------------------------------------------------------
    /// Checks if memroy region is accesible by CPU, when DMA ongoing.
    fn is_accessible(&self, addr: usize) -> bool {
        let src = if let Some(OamDma { src, .. }) = self.oam_dma {
            src
        } else {
            return true;
        };

        // TODO are registers accessible during DMA??
        // Only HRAM is accessible when DMA is ongoing for DMG.
        if in_ranges!(addr, ADDR_HRAM) {
            return true;
        }

        let is_wram_addr = |v| in_ranges!(v, ADDR_WRAM0, ADDR_WRAM1);
        // But for CGB, HRAM and either Cartridge or WRAM, whichever
        // is not a DMA source is also accesible.
        self.is_cgb
            && ((is_cart_addr(addr) != is_cart_addr(src))
                || (is_wram_addr(addr) != is_wram_addr(src)))
    }

    fn get_vram_wram_idx(&self) -> (usize, usize) {
        let start = *ADDR_IO_REGS.start();

        (
            self.regs[IO_VBK - start] as usize % VRAM_BANKS,
            self.regs[IO_SVBK - start] as usize % WRAM_BANKS,
        )
    }
}

impl Default for Mmu {
    fn default() -> Self {
        Self {
            is_cgb: false,
            cart: Cartidge::default(),
            oam_dma: None,

            vram: [[0; SIZE_VRAM_BANK]; VRAM_BANKS],
            wram: [[0; SIZE_WRAM_BANK]; WRAM_BANKS],
            oam: [0; SIZE_OAM],
            regs: [0; SIZE_IO_REGS],
            hram: [0; SIZE_HRAM],
            ie: 0,

            bg_palette: [0; SIZE_CGB_PALETTE],
            obj_palette: [0; SIZE_CGB_PALETTE],

            dpad: DPad::default(),
            buttons: ActionButtons::default(),
        }
    }
}

fn is_cart_addr(addr: usize) -> bool {
    in_ranges!(addr, ADDR_ROM0, ADDR_ROM1, ADDR_EXT_RAM)
}

/// Get ECHO RAM addres which is mapped to WRAM masked by 13-bits.
fn get_echo_ram_addr(rel_addr: usize) -> usize {
    (rel_addr & ECHO_RAM_ADDR_MASK) + *ADDR_WRAM0.start()
}

/// Combine `old` and `new`.
#[inline(always)]
fn combine(old: u8, new: u8, old_mask: u8) -> u8 {
    (old & old_mask) | (new & !old_mask)
}

#[inline(always)]
const fn mask(bit_cnt: u8) -> u8 {
    !(!0 << bit_cnt)
}
