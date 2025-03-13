use crate::{
    apu::Apu,
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
    // This property is duplicated in all components contained in it which
    // need it, because we do not want to use `Rc` and its good enough.
    pub(crate) is_2x: bool,

    pub(crate) ppu: Ppu,
    pub(crate) apu: Apu,
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
}

impl Mmu {
    pub(crate) fn new(cartd: Cartidge) -> Self {
        Self {
            is_2x: false,
            cart: cartd,

            ppu: Ppu::new(),
            apu: Apu::new(),
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
        }
    }

    pub(crate) fn tick(&mut self, mcycles: u32) {
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

        self.apu.tick(dots, self.timer.apu_ticks);
    }

    /// Reads one byte, use when executing instructions by CPU.
    pub(crate) fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;

        if is_cart_addr(addr) {
            return self.cart.read(addr);
        }

        match_range! { a@addr {
            ADDR_AUDIO_WAVE_RAM => { self.apu.ch3.wave_ram[a] }

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

        // Audio wave RAM is lies in the range of ADDR_IO_REGS,
        // so it must be before it otherwise we will lose writes to it.
        match_range! { a@addr {
            ADDR_AUDIO_WAVE_RAM => { self.apu.ch3.wave_ram[a] = val }

            ADDR_VRAM => { self.ppu.fetcher.vram[self.vram_idx][a] = val }
            ADDR_WRAM0 => { self.wram[0][a] = val}
            ADDR_WRAM1 => { self.wram[self.wram_idx][a] = val }
            ADDR_ECHO_RAM => { self.write(get_echo_ram_addr(a) as u16, val) }
            ADDR_OAM => { self.ppu.oam[a] = val }
            ADDR_UNUSABLE => {}
            ADDR_HRAM => { self.hram[a] = val}
            ADDR_IO_REGS => { self.write_reg(addr, val) }
            ADDR_IE => { self.write_reg(addr, val); }

            _ => { unreachable!() }
        }}
    }

    fn read_reg(&self, addr: usize) -> u8 {
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

            IO_NR10 => self.apu.ch1.nx0.read(),
            IO_NR11 => self.apu.ch1.nx1.read(),
            IO_NR12 => self.apu.ch1.nx2.read(),
            IO_NR13 => self.apu.ch1.nx3.period_low,
            IO_NR14 => self.apu.ch1.nx4.read(),
            IO_NR21 => self.apu.ch2.nx1.read(),
            IO_NR22 => self.apu.ch2.nx2.read(),
            IO_NR23 => self.apu.ch2.nx3.period_low,
            IO_NR24 => self.apu.ch2.nx4.read(),
            IO_NR30 => self.apu.ch3.n30.read(),
            IO_NR31 => self.apu.ch3.n31.length_period,
            IO_NR32 => self.apu.ch3.n32.read(),
            IO_NR33 => self.apu.ch3.n33.period_low,
            IO_NR34 => self.apu.ch3.n34.read(),
            IO_NR41 => self.apu.ch4.n41.read(),
            IO_NR42 => self.apu.ch4.n42.read(),
            IO_NR43 => self.apu.ch4.read_n43(),
            IO_NR44 => self.apu.ch4.n44.read(),
            IO_NR50 => self.apu.nr50.read(),
            IO_NR51 => self.apu.nr51.read(),
            IO_NR52 => self.apu.nr52.read(),

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
    fn write_reg(&mut self, addr: usize, v: u8) {
        /// Set value but keep the masked bits preserved.
        macro_rules! set {
            ($target:expr, $val:expr, $keep_mask:expr) => {{
                let combined = ($target.read() & $keep_mask) | ($val & !$keep_mask);
                $target.write(combined);
            }};
        }

        // pub(crate) const IO_WAVE_RAM: URange = 0xFF30..=0xFF3F;
        // Verify written data and perform the action.
        match addr {
            IO_JOYPAD => {
                set!(self.joypad, v, mask(4));
                self.update_joypad(self.dpad, self.buttons);
            }

            IO_SB => self.serial.sb = v,
            IO_SC => set!(self.serial.sc, v, mask(5) << 2),
            IO_DIV => self.timer.set_div(v),
            IO_TIMA => self.timer.tima = v,
            IO_TMA => self.timer.tma = v,
            IO_TAC => self.timer.tac.write(v),
            IO_IF => set!(self.iflag, v, !mask(5)),
            IO_IE => set!(self.ienable, v, !mask(5)),

            IO_NR10 => set!(self.apu.ch1.nx0, v, 1 << 7),
            IO_NR11 => self.apu.ch1.nx1.write(v),
            IO_NR12 => self.apu.ch1.nx2.write(v),
            IO_NR13 => self.apu.ch1.nx3.period_low = v,
            IO_NR14 => set!(self.apu.ch1.nx4, v, mask(3) << 3),

            IO_NR21 => self.apu.ch2.nx1.write(v),
            IO_NR22 => self.apu.ch2.nx2.write(v),
            IO_NR23 => self.apu.ch2.nx3.period_low = v,
            IO_NR24 => set!(self.apu.ch2.nx4, v, mask(3) << 3),

            IO_NR30 => set!(self.apu.ch3.n30, v, mask(7)),
            IO_NR31 => self.apu.ch3.n31.length_period = v,
            IO_NR32 => set!(self.apu.ch3.n32, v, 1 << 7 | mask(5)),
            IO_NR33 => self.apu.ch3.n33.period_low = v,
            IO_NR34 => set!(self.apu.ch3.n34, v, mask(3) << 3),

            IO_NR41 => set!(self.apu.ch4.n41, v, mask(2) << 6),
            IO_NR42 => self.apu.ch4.n42.write(v),
            IO_NR43 => self.apu.ch4.write_n43(v),
            IO_NR44 => set!(self.apu.ch4.n44, v, mask(6)),

            IO_NR50 => self.apu.nr50.write(v),
            IO_NR51 => self.apu.nr51.write(v),
            IO_NR52 => set!(self.apu.nr52, v, mask(7)),

            IO_LCDC => self.ppu.fetcher.lcdc.write(v),
            IO_STAT => set!(self.ppu.stat, v, mask(3)),
            IO_SCY => self.ppu.fetcher.scy = v,
            IO_SCX => self.ppu.fetcher.scx = v,
            IO_LY => (),
            IO_LYC => self.ppu.lyc = v,
            IO_WY => self.ppu.fetcher.wy = v,
            IO_WX => self.ppu.fetcher.wx = v,
            IO_BGP => self.ppu.bgp = v,
            IO_OBP0 => self.ppu.obp0 = v,
            IO_OBP1 => self.ppu.obp1 = v,
            IO_BGPI => self.bgpi.write(v),
            IO_OBPI => self.obpi.write(v),

            // CGB paletes are locked during when PPU is drawing(Mode-3).
            IO_BGPD if self.get_mode() != MODE_DRAW => {
                self.ppu.bg_palette[self.bgpi.addr as usize] = v;
                if self.bgpi.auto_inc == 1 {
                    self.bgpi.addr = (self.bgpi.addr + 1) & mask(6);
                }
            }
            IO_OBPD if self.get_mode() != MODE_DRAW => {
                self.ppu.obj_palette[self.obpi.addr as usize] = v;
                if self.obpi.auto_inc == 1 {
                    self.obpi.addr = (self.obpi.addr + 1) & mask(6);
                }
            }

            IO_OPRI => self.opri = v & 1,
            IO_SVBK => {
                if self.is_2x {
                    self.wram_idx = if v == 0 { 1 } else { (v & mask(3)) as usize };
                }
            }
            IO_VBK => {
                if self.is_2x {
                    self.vram_idx = (v as usize) & 1
                }
            }

            // IO_HDMA1 => { = val}
            // IO_HDMA2 => { = val}
            // IO_HDMA3 => { = val}
            // IO_HDMA4 => { = val}
            // IO_HDMA5 => { = val}
            IO_DMA => self.do_dma(v),
            IO_KEY1 => set!(self.key1, v, !mask(1)),
            IO_RP => set!(self.rp, v, 1 << 1),

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

    pub(crate) fn get_mode(&self) -> u8 {
        self.ppu.stat.ppu_mode
    }

    fn do_dma(&mut self, addr: u8) {
        // DMA address specifies the high-byte value of the 16-bit
        // source address. Valid values for it are from 0x00 to 0xDF.
        // If it is more than that then we just wrap around it.
        let src = ((addr as usize) % (0xDF + 1)) << 8;
        self.dma = addr;

        for (i, _) in ADDR_OAM.enumerate() {
            self.ppu.oam[i] = self.read((src + i) as u16);
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
