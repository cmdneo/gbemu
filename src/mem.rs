use crate::{
    cartridge::Cartidge,
    info::*,
    macros::{in_ranges, match_range},
    ppu::Ppu,
    regs::{ActionButtons, CgbPaletteIndex, DPad, IntrBits, JoyPad, Key1, Rp},
    serial::Serial,
    timer::Timer,
};

/// The memory sub-system, contains the `Cartridge`, `Ppu`, `Timer`, `Serial`
/// and some registers, other registers are owned by components they belong to.
pub(crate) struct Mmu {
    /// Is running in dual-speed(aka CGB mode).
    /// This property is replicated by all components contained by it.
    pub(crate) is_2x: bool,
    pub(crate) ppu: Ppu,
    pub(crate) timer: Timer,
    pub(crate) serial: Serial,
    pub(crate) cart: Cartidge,

    // Registers and memory owned by it.
    pub(crate) key1: Key1,
    pub(crate) iflag: IntrBits,
    pub(crate) ienable: IntrBits,
    pub(crate) joypad: JoyPad,
    pub(crate) bgpi: CgbPaletteIndex,
    pub(crate) obpi: CgbPaletteIndex,
    pub(crate) opri: u8,
    pub(crate) dma: u8,
    pub(crate) rp: Rp,
    pub(crate) wram_idx: usize,
    pub(crate) vram_idx: usize,
    // First WRAM region always refers to bank-0 and
    // second WRAM region can refer to any of the 1-7 banks.
    wram: [[u8; SIZE_WRAM_BANK]; WRAM_BANKS],
    hram: [u8; SIZE_HRAM],

    dpad: DPad,
    buttons: ActionButtons,
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
        Self {
            wram_idx: 1,
            cart: cartd,
            ..Default::default()
        }
    }

    pub(crate) fn tick(&mut self, mcycles: u16) {
        // Dual-speed mode does not change PPU or Audio speed.
        let dots = if self.is_2x { mcycles * 2 } else { mcycles * 4 };

        let intr = self.ppu.tick(dots);
        self.add_interrupt(intr);
        if self.timer.tick(mcycles) {
            self.iflag.timer = 1;
        }
        if self.serial.tick(mcycles, self.cart.is_cgb) {
            self.iflag.serial = 1;
        }

        let mut dma = if let Some(d) = self.oam_dma {
            d
        } else {
            return;
        };

        for _ in 0..mcycles {
            if dma.copied == dma.count {
                break;
            }

            let addr = dma.src + dma.copied;
            self.ppu.oam[dma.copied] = self.read(addr as u16);
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
    pub(crate) fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;

        if is_cart_addr(addr) {
            return self.cart.read(addr);
        }

        match_range! { a@addr {
            ADDR_VRAM => { self.ppu.fetcher.vram[self.vram_idx][a] }
            ADDR_WRAM0 => { self.wram[0][a] }
            ADDR_WRAM1 => { self.wram[self.wram_idx][a] }
            ADDR_ECHO_RAM => { self.read(get_echo_ram_addr(a) as u16) }
            ADDR_OAM => { self.ppu.oam[a] }
            ADDR_UNUSABLE => { 0 }
            ADDR_HRAM => { self.hram[a] }
            ADDR_IO_REGS => { self.read_reg(addr) }
            ADDR_IE => { self.read_reg(addr) }

            _ => { unreachable!() }
        }}
    }

    /// Writes one byte, use when executing instructions by CPU.
    /// Writes to read-only registers are ignored, use `reg_set` for that.    timer:

    pub(crate) fn write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;

        if is_cart_addr(addr) {
            self.cart.write(addr, val);
            return;
        }

        let mode = self.ppu.stat.ppu_mode;
        // Ignore writes to graphics related memory regions which are
        // inaccessible during certain PPU modes.
        match_range! { a@addr {
            ADDR_VRAM => {
                // FIXME Fix this prevents test ROMs from fully writing DATA.
                if mode != MODE_DRAW {
                    self.ppu.fetcher.vram[self.vram_idx][a] = val
                }
            }
            ADDR_WRAM0 => { self.wram[0][a] = val}
            ADDR_WRAM1 => { self.wram[self.wram_idx][a] = val }
            ADDR_ECHO_RAM => { self.write(get_echo_ram_addr(a) as u16, val) }

            ADDR_OAM => {
                if mode != MODE_DRAW && mode != MODE_SCAN {
                    self.ppu.oam[a] = val
                }
            }

            ADDR_UNUSABLE => {}
            ADDR_HRAM => { self.hram[a] = val}
            ADDR_IO_REGS => { self.write_reg(addr, val) }
            ADDR_IE => { self.write_reg(addr, val); }

            _ => { unreachable!() }
        }}
    }

    fn read_reg(&self, addr: usize) -> u8 {
        // pub(crate) const IO_WAVE_RAM: URange = 0xFF30..=0xFF3F;

        match addr {
            IO_JOYPAD => self.joypad.read(),
            IO_SB => self.serial.sb,
            IO_SC => self.serial.sc.read(),
            IO_DIV => self.timer.get_div(),
            IO_TIMA => self.timer.tima,
            IO_TMA => self.timer.tma,
            IO_TAC => self.timer.tac.read(),
            IO_IF => self.iflag.read(),
            IO_IE => self.ienable.read(),
            // IO_NR10 => {}
            // IO_NR11 => {}
            // IO_NR12 => {}
            // IO_NR13 => {}
            // IO_NR14 => {}
            // IO_NR21 => {}
            // IO_NR22 => {}
            // IO_NR23 => {}
            // IO_NR24 => {}
            // IO_NR30 => {}
            // IO_NR31 => {}
            // IO_NR32 => {}
            // IO_NR33 => {}
            // IO_NR34 => {}
            // IO_NR41 => {}
            // IO_NR42 => {}
            // IO_NR43 => {}
            // IO_NR44 => {}
            // IO_NR50 => {}
            // IO_NR51 => {}
            // IO_NR52 => {}
            // IO_PCM12 => {}
            // IO_PCM34 => {}
            IO_LCDC => self.ppu.fetcher.lcdc.read(),
            IO_STAT => self.ppu.stat.read(),
            IO_SCY => self.ppu.fetcher.scy,
            IO_SCX => self.ppu.fetcher.scx,
            IO_LY => self.ppu.ly,
            IO_LYC => self.ppu.lyc,
            IO_WY => self.ppu.fetcher.wy,
            IO_WX => self.ppu.fetcher.wx,
            IO_BGP => self.ppu.bgp,
            IO_OBP0 => self.ppu.obp0,
            IO_OBP1 => self.ppu.obp1,
            IO_BGPI => self.bgpi.read(),
            IO_BGPD => self.ppu.bg_palette[self.bgpi.addr as usize],
            IO_OBPI => self.obpi.read(),
            IO_OBPD => self.ppu.obj_palette[self.obpi.addr as usize],
            IO_OPRI => self.opri,
            IO_SVBK => self.wram_idx as u8,
            IO_VBK => self.vram_idx as u8,
            // IO_HDMA1 => {}
            // IO_HDMA2 => {}
            // IO_HDMA3 => {}
            // IO_HDMA4 => {}
            // IO_HDMA5 => {}
            IO_DMA => self.dma,
            IO_KEY1 => self.key1.read(),
            IO_RP => self.rp.read(),

            _ => 0,
        }
    }

    /// Writes to a register and performs necessary action
    /// corresponding to the register if any.
    ///
    /// Writes to read-only registers(or register fields) are ignored.
    fn write_reg(&mut self, addr: usize, val: u8) {
        // Set value but keep masked bits preserved(if mask present).
        macro_rules! set {
            ($target:expr, $val:expr, $keep_mask:expr) => {{
                let combined = ($target.read() & $keep_mask) | ($val & !$keep_mask);
                $target.write(combined);
            }};
            ($target:expr, $val:expr) => {
                $target.write($val)
            };
        }

        // pub(crate) const IO_WAVE_RAM: URange = 0xFF30..=0xFF3F;
        // Verify written data and perform the action.
        match addr {
            IO_JOYPAD => {
                set!(self.joypad, val, mask(4));
                self.update_joypad(self.dpad, self.buttons);
            }
            IO_SB => self.serial.sb = val,
            IO_SC => set!(self.serial.sc, val, mask(5) << 2),
            IO_DIV => self.timer.set_div(val),
            IO_TIMA => self.timer.tima = val,
            IO_TMA => self.timer.tma = val,
            IO_TAC => self.timer.tac.write(val),
            IO_IF => set!(self.iflag, val, !mask(5)),
            IO_IE => set!(self.ienable, val, !mask(5)),
            // IO_NR10 => { = val}
            // IO_NR11 => { = val}
            // IO_NR12 => { = val}
            // IO_NR13 => { = val}
            // IO_NR14 => { = val}
            // IO_NR21 => { = val}
            // IO_NR22 => { = val}
            // IO_NR23 => { = val}
            // IO_NR24 => { = val}
            // IO_NR30 => { = val}
            // IO_NR31 => { = val}
            // IO_NR32 => { = val}
            // IO_NR33 => { = val}
            // IO_NR34 => { = val}
            // IO_NR41 => { = val}
            // IO_NR42 => { = val}
            // IO_NR43 => { = val}
            // IO_NR44 => { = val}
            // IO_NR50 => { = val}
            // IO_NR51 => { = val}
            // IO_NR52 => set!(self.audio.nr52, val, mask(7)),
            IO_PCM12 => (),
            IO_PCM34 => (),
            IO_LCDC => set!(self.ppu.fetcher.lcdc, val),
            IO_STAT => set!(self.ppu.stat, val, mask(3)),
            IO_SCY => self.ppu.fetcher.scy = val,
            IO_SCX => self.ppu.fetcher.scx = val,
            IO_LY => (),
            IO_LYC => self.ppu.lyc = val,
            IO_WY => self.ppu.fetcher.wy = val,
            IO_WX => self.ppu.fetcher.wx = val,
            IO_BGP => self.ppu.bgp = val,
            IO_OBP0 => self.ppu.obp0 = val,
            IO_OBP1 => self.ppu.obp1 = val,
            IO_BGPI => set!(self.bgpi, val),
            IO_OBPI => self.obpi.write(val),

            // CGB paletes are locked during when PPU is drawing(Mode-3).
            IO_BGPD if self.get_mode() != MODE_DRAW => {
                self.ppu.bg_palette[self.bgpi.addr as usize] = val;
                if self.bgpi.auto_inc == 1 {
                    self.bgpi.addr = (self.bgpi.addr + 1) & mask(6);
                }
            }
            IO_OBPD if self.get_mode() != MODE_DRAW => {
                self.ppu.obj_palette[self.obpi.addr as usize] = val;
                if self.obpi.auto_inc == 1 {
                    self.obpi.addr = (self.obpi.addr + 1) & mask(6);
                }
            }

            IO_OPRI => self.opri = val & 1,
            IO_SVBK if self.is_2x => {
                if val == 0 {
                    self.wram_idx = 1;
                } else {
                    self.wram_idx = (val & mask(3)) as usize;
                }
            }
            IO_VBK if self.is_2x => self.vram_idx = (val as usize) & 1,

            // IO_HDMA1 => { = val}
            // IO_HDMA2 => { = val}
            // IO_HDMA3 => { = val}
            // IO_HDMA4 => { = val}
            // IO_HDMA5 => { = val}
            IO_DMA => self.start_dma(val),
            IO_KEY1 => set!(self.key1, val, !mask(1)),
            IO_RP => set!(self.rp, val, 1 << 1),

            _ => (),
        }
    }

    /// Set IF register by ORing bits of `iflag` in.
    pub(crate) fn add_interrupt(&mut self, iflag: IntrBits) {
        let val = self.iflag.read() | iflag.read();
        self.iflag.write(val);
    }

    /// Update joypad buttons and Joypad/P1 register.
    /// Also, raise Joypad interrupt if condition is met.
    pub(crate) fn update_joypad(&mut self, dpad: DPad, btns: ActionButtons) {
        let mut new = mask(4); // In Joypad 0-bit means pressed.

        if self.joypad.select_dpad == 0 {
            new &= !dpad.read();
        }
        if self.joypad.select_buttons == 0 {
            new &= !btns.read();
        }

        // Interrupt only when any of the lower 4-bits of Joypad falls.
        if (self.joypad.state & !new) & mask(4) != 0 {
            self.add_interrupt(IntrBits {
                joypad: 1,
                ..Default::default()
            });
        }

        self.joypad.state = new;
        self.dpad = dpad;
        self.buttons = btns;
    }

    /// Get `IF & IE` as `IntData`.
    pub(crate) fn get_queued_ints(&self) -> IntrBits {
        IntrBits::new(self.iflag.read() & self.ienable.read())
    }

    pub(crate) fn get_mode(&self) -> u8 {
        self.ppu.stat.ppu_mode
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

        self.dma = addr;
    }

    // / Checks if memroy region is accesible by CPU, when DMA ongoing.
    // fn is_accessible(&self, addr: usize) -> bool {
    //     let src = if let Some(OamDma { src, .. }) = self.oam_dma {
    //         src
    //     } else {
    //         return true;
    //     };

    //     // Only HRAM is accessible when DMA is ongoing for DMG.
    //     if in_ranges!(addr, ADDR_HRAM) {
    //         return true;
    //     }

    //     let is_wram_addr = |v| in_ranges!(v, ADDR_WRAM0, ADDR_WRAM1);
    //     // But for CGB, HRAM and either Cartridge or WRAM, whichever
    //     // is not a DMA source is also accesible.
    //     self.is_2x
    //         && ((is_cart_addr(addr) != is_cart_addr(src))
    //             || (is_wram_addr(addr) != is_wram_addr(src)))
    // }
}

impl Default for Mmu {
    fn default() -> Self {
        Self {
            is_2x: false,
            cart: Default::default(),

            ppu: Ppu::new(),
            timer: Timer::new(),
            serial: Serial::new(),

            wram: [[0; SIZE_WRAM_BANK]; WRAM_BANKS],
            hram: [0; SIZE_HRAM],
            ienable: Default::default(),
            iflag: Default::default(),
            key1: Default::default(),
            joypad: Default::default(),
            bgpi: Default::default(),
            obpi: Default::default(),
            wram_idx: 1,
            vram_idx: 0,
            opri: 0,
            dma: 0,
            rp: Rp::new(0b10),

            dpad: Default::default(),
            buttons: Default::default(),
            oam_dma: None,
        }
    }
}

#[inline]
fn is_cart_addr(addr: usize) -> bool {
    in_ranges!(addr, ADDR_ROM0, ADDR_ROM1, ADDR_EXT_RAM)
}

/// Get ECHO RAM addres which is mapped to WRAM masked by 13-bits.
#[inline]
fn get_echo_ram_addr(rel_addr: usize) -> usize {
    (rel_addr & ECHO_RAM_ADDR_MASK) + *ADDR_WRAM0.start()
}

#[inline(always)]
const fn mask(bit_cnt: u32) -> u8 {
    u8::MAX >> (8 - bit_cnt)
}
