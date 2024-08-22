mod fetcher;
mod tile;

use fetcher::{LineFetcher, Pixel};
use tile::OamEntry;

use crate::{
    display::{self, Color, Frame},
    info::*,
    mem::Mmu,
    regs::{CgbPaletteColor, IntData, LcdCtrl, LcdStat},
};

#[derive(Default)]
pub(crate) struct Ppu {
    /// Current PPU mode from STAT register.
    mode: PpuMode,
    /// Frame containing an RGB-24 representation of the screen pixels.
    frame: Frame,
    /// Line fetcher.
    fetcher: LineFetcher,

    /// Amount of dots left to goto the next mode.
    /// In normal mode     : 4 dots per M-cycle.
    /// In dual-speed mode : 2 dots per M-cycle.
    dots_left: u32,
    /// Number of dots consumed for the current scan-line.
    line: ScanLine,
}

#[derive(Default)]
struct ScanLine {
    /// Dots, always a multiple of 2.
    dots: u32,
    /// Scan-line number
    y: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum PpuMode {
    HBlank = MODE_HBLANK,
    VBlank = MODE_VBLANK,
    #[default]
    Scan = MODE_SCAN,
    Draw = MODE_DRAW,
}

// TODO Allow PPU and LCD disabling, via lcdc bit 7.
impl Ppu {
    pub(crate) fn new() -> Self {
        Self {
            mode: PpuMode::Scan,
            fetcher: LineFetcher::new(),
            dots_left: 0,
            ..Default::default()
        }
    }

    pub(crate) fn fill_frame(&self, frame: &mut display::Frame) {
        *frame = self.frame.clone();
    }

    /// Run for `dots` cycles, `dots` must be an even number.
    pub(crate) fn tick(&mut self, mmu: &mut Mmu, dots: u32) {
        // Reset and do nothing if PPU is disabled.
        let lcdc = LcdCtrl::new(mmu.read(IO_LCDC));
        if lcdc.ppu_enable == 0 {
            self.reset(mmu);
            return;
        }

        assert!(dots % 2 == 0);
        self.dots_left += dots;

        while self.dots_left > 0 {
            let mode = match self.mode {
                PpuMode::HBlank => self.step_hblank(),
                PpuMode::VBlank => self.step_vblank(),
                PpuMode::Scan => self.step_scan(mmu),
                PpuMode::Draw => self.step_draw(mmu),
            };

            self.update_lcd_state(mmu, mode);
        }
    }

    fn reset(&mut self, mmu: &mut Mmu) {
        // On PPU & LCD disable no interrupts are generated.
        let mut stat = LcdStat::new(mmu.read(IO_STAT));
        stat.ppu_mode = 0;

        mmu.set_reg(IO_STAT, stat.read());
        mmu.set_reg(IO_LY, 0);

        self.line = Default::default();
        self.mode = PpuMode::Scan;
    }

    fn step_scan(&mut self, mmu: &mut Mmu) -> PpuMode {
        // 2 dots per entry scan. Lasts 80 dots for scanning 40 entries.
        let idx = self.line.dots as usize / 2;
        match idx {
            0 => self.fetcher.new_line(mmu, self.line.y),
            OAM_ENTRIES => return PpuMode::Draw,
            _ => (),
        }

        self.eat_dots(2);
        let obj = get_oam_entry(mmu, idx);

        // If the spte buffer is not full, then a sprite is added to it if:
        // It is on the scan-line as per its Y-pos and objects are enabled.
        // "Ypos - 16" is sprite top position on screen.
        // A sprite can have size: 8x8 or 8x16.
        let lcdc = LcdCtrl::new(mmu.read(IO_LCDC));
        let height = if lcdc.obj_size == 1 { 16 } else { 8 };
        if self.fetcher.objects.len() < MAX_OBJ_PER_LINE
            && obj.ypos <= self.line.y + 16
            && self.line.y + 16 < obj.ypos + height
        {
            self.fetcher.objects.push(obj);
        }

        self.mode
    }

    fn step_draw(&mut self, mmu: &mut Mmu) -> PpuMode {
        self.eat_dots(2);
        self.fetcher.tick_2_dots(mmu);

        if self.fetcher.is_done() {
            // Copy all pixel colors to frame if done.
            for i in 0..SCREEN_RESOLUTION.0 {
                let px = self.fetcher.screen_line[i];
                let color = pixel_to_color(mmu, px);
                self.frame.set(i, self.line.y as usize, color);
            }

            PpuMode::HBlank
        } else {
            self.mode
        }
    }

    fn step_hblank(&mut self) -> PpuMode {
        // TODO goto Scan directly if reset detected??
        // If current scan-line finishes and it was last draw line then
        // goto VBlank, if not last line then just go back to OAM-Scan mode.
        if self.eat_dots(self.dots_left) {
            if self.line.y == PPU_DRAW_LINES {
                PpuMode::VBlank
            } else {
                PpuMode::Scan
            }
        } else {
            self.mode
        }
    }

    fn step_vblank(&mut self) -> PpuMode {
        self.eat_dots(self.dots_left);

        if self.line.y == PPU_DRAW_LINES + PPU_VBLANK_LINES {
            self.line.dots = 0;
            self.line.y = 0;
            PpuMode::Scan // Start next frame.
        } else {
            self.mode
        }
    }

    /// Update STAT and LY registers and raise interrupts if any.
    /// Must be called after each step.
    fn update_lcd_state(&mut self, mmu: &mut Mmu, new_mode: PpuMode) {
        let mut s = LcdStat::new(mmu.read(IO_STAT));
        let mut iflag = IntData::new(mmu.read(IO_IF));
        let lyc = mmu.read(IO_LYC);

        s.ppu_mode = new_mode as u8;
        s.ly_eq_lyc = (lyc == self.line.y) as u8;

        // For interrupt on condition: LYC == LY.
        // It is trigerred at the begining of a scan line only.
        if self.line.dots == 0 && s.lyc_int == 1 && s.ly_eq_lyc == 1 {
            iflag.stat = 1;
        }
        // If mode changes and interrupt condition is met then interrupt.
        if new_mode != self.mode {
            iflag.vblank = matches!(new_mode, PpuMode::VBlank) as u8;

            iflag.stat = match self.mode {
                PpuMode::HBlank if s.mode0_int == 1 => 1,
                PpuMode::VBlank if s.mode1_int == 1 => 1,
                PpuMode::Scan if s.mode2_int == 1 => 1,
                _ => iflag.stat,
            };
        }

        self.mode = new_mode;
        mmu.set_reg(IO_LY, self.line.y);
        mmu.set_reg(IO_IF, iflag.read());
        mmu.set_reg(IO_STAT, s.read());
    }

    /// Consume as much dots as possible from `dots_left` without overflowing
    /// into the next scan-line. Return true if current scan-line finished.
    fn eat_dots(&mut self, dots: u32) -> bool {
        assert!(dots <= PPU_HSCAN_DOTS);
        assert!(dots <= self.dots_left);
        let r = self.line.dots + dots;

        if r >= PPU_HSCAN_DOTS {
            // Consume only as many dots as needed to finish this line.
            self.dots_left -= dots - (r - PPU_HSCAN_DOTS);
            self.line.dots = 0;
            self.line.y += 1;
            true
        } else {
            self.line.dots += dots;
            self.dots_left -= dots;
            false
        }
    }
}

fn pixel_to_color(mmu: &mut Mmu, px: Pixel) -> Color {
    // In non-CGB mode palette is taken from BGP/OBP0/OBP1 registers,
    // where colors are stored according to color IDs as: [MSB] 33-22-11-00 [LSB]
    let mono_color = |palette, color_id| (palette >> color_id * 2) & 0b11;

    if mmu.is_cgb {
        // Transparent[color=0] object pixels have already been
        // handeled by the fetcher during pixel mixing.
        let palette = read_cgb_palette(mmu, px.is_obj, px.palette);
        cgb_to_color(palette[px.color_id as usize])
    } else {
        let pal_reg = if px.is_obj {
            if px.palette == 0 {
                IO_OBP0
            } else {
                IO_OBP1
            }
        } else {
            IO_BGP
        };

        let color = mono_color(mmu.read(pal_reg), px.color_id);
        mono_to_color(color)
    }
}

fn read_cgb_palette(mmu: &Mmu, is_obj: bool, pal_index: u8) -> [u16; 4] {
    let mut ret = [0u16; 4];

    for (i, r) in ret.iter_mut().enumerate() {
        // Each palette is of 8-bytes consisting of 4 colors of 2-bytes each.
        let idx = (pal_index as usize) * 8 + i * 2;

        *r = u16::from_le_bytes(if is_obj {
            [mmu.obj_palette[idx], mmu.obj_palette[idx + 1]]
        } else {
            [mmu.bg_palette[idx], mmu.bg_palette[idx + 1]]
        });
    }

    ret
}

fn get_oam_entry(mmu: &mut Mmu, idx: usize) -> OamEntry {
    let d = &mmu.oam[(idx * 4)..(idx * 4 + 4)];
    OamEntry::from_array([d[0], d[1], d[2], d[3]])
}

#[inline]
fn mono_to_color(mono_color: u8) -> Color {
    // Each Color component is of 8 bits and mono of 2 bits.
    // Where in mono color: 3 in means dark and 0 white.
    const SCALE: u8 = 255 / 3;
    let c = (3 - mono_color) * SCALE;
    Color { r: c, g: c, b: c }
}

#[inline]
fn cgb_to_color(cgb_color: u16) -> Color {
    // Each Color component is of 8 bits and CGB color component of 5 bits.
    const SCALE: u8 = 255 / 31;
    let c = CgbPaletteColor::new(cgb_color);
    Color {
        r: (c.red as u8) * SCALE,
        g: (c.green as u8) * SCALE,
        b: (c.blue as u8) * SCALE,
    }
}
