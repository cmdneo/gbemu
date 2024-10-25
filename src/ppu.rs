mod fetcher;

use fetcher::{LineFetcher, OamEntry, Pixel};

use crate::{
    frame::{self, Color, Frame},
    info::*,
    regs::{CgbPaletteColor, IntrBits, LcdStat},
};

pub(crate) struct Ppu {
    pub(crate) fetcher: LineFetcher,

    // Memory and registers owned by it.
    pub(crate) oam: [u8; SIZE_OAM],
    // CGB color palettes are stored in a seperate RAM accesed indirectly.
    pub(crate) bg_palette: [u8; SIZE_CGB_PALETTE],
    pub(crate) obj_palette: [u8; SIZE_CGB_PALETTE],
    pub(crate) stat: LcdStat,
    pub(crate) ly: u8,
    pub(crate) lyc: u8,
    pub(crate) bgp: u8,
    pub(crate) obp0: u8,
    pub(crate) obp1: u8,

    /// Current PPU mode updates to it are carried to STAT register.
    mode: PpuMode,
    /// Frame containing an RGB-24 representation of the screen pixels.
    frame: Frame,
    /// Amount of dots left, which determines how much to advance.
    /// In normal mode     : 4 dots per M-cycle.
    /// In dual-speed mode : 2 dots per M-cycle.
    dots_left: u16,
    /// Number of dots consumed for the current scan-line `LY`.
    dots_in_line: u16,
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

impl Ppu {
    pub(crate) fn new() -> Self {
        Self {
            fetcher: LineFetcher::new(),
            oam: [0; SIZE_OAM],
            bg_palette: [0; SIZE_CGB_PALETTE],
            obj_palette: [0; SIZE_CGB_PALETTE],
            stat: Default::default(),
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            frame: Default::default(),
            mode: PpuMode::Scan,
            dots_in_line: 0,
            dots_left: 0,
        }
    }

    pub(crate) fn fill_frame(&self, frame: &mut frame::Frame) {
        *frame = self.frame.clone();
    }

    /// Run for `dots` cycles, `dots` must be an even number.
    pub(crate) fn tick(&mut self, dots: u16) -> IntrBits {
        // Reset and do nothing if PPU is disabled.
        if self.fetcher.lcdc.ppu_enable == 0 {
            self.reset();
            return IntrBits::new(0);
        }

        assert!(dots % 2 == 0);
        self.dots_left += dots;
        let mut ret = IntrBits::default();

        while self.dots_left > 0 {
            let mode = match self.mode {
                PpuMode::HBlank => self.step_hblank(),
                PpuMode::VBlank => self.step_vblank(),
                PpuMode::Scan => self.step_scan(),
                PpuMode::Draw => self.step_draw(),
            };

            let new_intrps = self.update_lcd_state(mode);
            ret.write(ret.read() | new_intrps.read());
        }

        ret
    }

    fn reset(&mut self) {
        self.stat.ppu_mode = MODE_HBLANK;
        self.ly = 0;
        self.dots_in_line = 0;
        self.mode = PpuMode::Scan;
    }

    fn step_scan(&mut self) -> PpuMode {
        // 2 dots per entry scan. Lasts 80 dots for scanning 40 entries.
        let idx = self.dots_in_line as usize / 2;
        match idx {
            0 => self.fetcher.new_line(self.ly),
            OAM_ENTRIES => return PpuMode::Draw,
            _ => (),
        }

        self.eat_dots(2);
        let obj = get_oam_entry(&self.oam, idx);

        // If the spte buffer is not full, then a sprite is added to it if:
        // It is on the scan-line as per its Y-pos and objects are enabled.
        // "Ypos - 16" is sprite top position on screen.
        // A sprite can have size: 8x8 or 8x16(tall object mode).
        let height = if self.fetcher.lcdc.obj_size == 1 {
            16
        } else {
            8
        };
        if self.fetcher.objects.len() < MAX_OBJ_PER_LINE
            && obj.ypos <= self.ly + 16
            && self.ly + 16 < obj.ypos + height
        {
            self.fetcher.objects.push(obj);
        }

        PpuMode::Scan
    }

    fn step_draw(&mut self) -> PpuMode {
        self.eat_dots(2);
        self.fetcher.tick_2_dots();

        if self.fetcher.is_done() {
            // Copy all pixel colors to frame if done.
            for i in 0..SCREEN_RESOLUTION.0 {
                let px = self.fetcher.screen_line[i];
                let color = self.pixel_to_color(px);
                self.frame.set(i, self.ly as usize, color);
            }

            PpuMode::HBlank
        } else {
            PpuMode::Draw
        }
    }

    fn step_hblank(&mut self) -> PpuMode {
        // If current scan-line finishes and it was last draw line then
        // goto VBlank, if not last line then just go back to OAM-Scan mode.
        if self.eat_dots(self.dots_left) {
            if self.ly == PPU_DRAW_LINES {
                PpuMode::VBlank
            } else {
                PpuMode::Scan
            }
        } else {
            PpuMode::HBlank
        }
    }

    fn step_vblank(&mut self) -> PpuMode {
        self.eat_dots(self.dots_left);

        if self.ly == PPU_DRAW_LINES + PPU_VBLANK_LINES {
            self.dots_in_line = 0;
            self.ly = 0;
            PpuMode::Scan // Start next frame.
        } else {
            PpuMode::VBlank
        }
    }

    /// Update STAT and LY registers and raise interrupts if any.
    /// Must be called after each step.
    fn update_lcd_state(&mut self, new_mode: PpuMode) -> IntrBits {
        let mut iflag = IntrBits::new(0);

        // For interrupt on condition: LYC == LY.
        // It is trigerred at the begining of a scan line only.
        if self.dots_in_line == 0 && self.stat.lyc_int == 1 && self.lyc == self.ly {
            iflag.stat = 1;
        }
        // If mode changes and interrupt condition is met then interrupt.
        if new_mode != self.mode {
            iflag.vblank = matches!(new_mode, PpuMode::VBlank) as u8;
            iflag.stat = match self.mode {
                PpuMode::HBlank if self.stat.mode0 == 1 => 1,
                PpuMode::VBlank if self.stat.mode1 == 1 => 1,
                PpuMode::Scan if self.stat.mode2 == 1 => 1,
                _ => iflag.stat,
            };
        }

        self.stat.ppu_mode = new_mode as u8;
        self.stat.ly_eq_lyc = (self.lyc == self.ly) as u8;
        self.mode = new_mode;
        iflag
    }

    /// Consume as much dots as possible from `dots_left` without overflowing
    /// into the next scan-line. Return true if current scan-line finished.
    fn eat_dots(&mut self, dots: u16) -> bool {
        assert!(dots <= PPU_HSCAN_DOTS);
        assert!(dots <= self.dots_left);
        let r = self.dots_in_line + dots;

        if r >= PPU_HSCAN_DOTS {
            // Consume only as many dots as needed to finish this line.
            self.dots_left -= dots - (r - PPU_HSCAN_DOTS);
            self.dots_in_line = 0;
            self.ly += 1;
            true
        } else {
            self.dots_in_line += dots;
            self.dots_left -= dots;
            false
        }
    }

    // Pixel to color synthesis stuff
    //---------------------------------------------------------------
    fn pixel_to_color(&self, px: Pixel) -> Color {
        // In non-CGB mode palette is taken from BGP/OBP0/OBP1 registers,
        // where colors are stored according to color IDs as: [MSB] 33-22-11-00 [LSB]
        let mono_color = |palette, color_id| (palette >> color_id * 2) & 0b11;

        if self.fetcher.is_2x {
            // Transparent[color=0] object pixels have already been
            // handeled by the fetcher during pixel mixing.
            let palette = self.read_cgb_palette(px.is_obj, px.palette);
            cgb_to_color(palette[px.color_id as usize])
        } else {
            let palette = if px.is_obj {
                if px.palette == 0 {
                    self.obp0
                } else {
                    self.obp1
                }
            } else {
                self.bgp
            };

            let color = mono_color(palette, px.color_id);
            mono_to_color(color)
        }
    }

    fn read_cgb_palette(&self, is_obj: bool, pal_index: u8) -> [u16; 4] {
        let mut ret = [0u16; 4];

        for (i, r) in ret.iter_mut().enumerate() {
            // Each palette is of 8-bytes consisting of 4 colors of 2-bytes each.
            let idx = (pal_index as usize) * 8 + i * 2;

            *r = u16::from_le_bytes(if is_obj {
                [self.obj_palette[idx], self.obj_palette[idx + 1]]
            } else {
                [self.bg_palette[idx], self.bg_palette[idx + 1]]
            });
        }

        ret
    }
}

fn get_oam_entry(oam: &[u8], idx: usize) -> OamEntry {
    let d = &oam[(idx * 4)..(idx * 4 + 4)];
    OamEntry::from_array([d[0], d[1], d[2], d[3]])
}

#[inline]
fn mono_to_color(mono_color: u8) -> Color {
    // Mono color is of 2 bits.
    // Where in mono color: 3 in it means dark and 0 white.
    const SCALE: u8 = 255 / 3;
    let c = (3 - mono_color) * SCALE;
    Color { r: c, g: c, b: c }
}

#[inline]
fn cgb_to_color(cgb_color: u16) -> Color {
    // Each CGB color component of 5 bits.
    const SCALE: u8 = 255 / 31;
    let c = CgbPaletteColor::new(cgb_color);
    Color {
        r: (c.red as u8) * SCALE,
        g: (c.green as u8) * SCALE,
        b: (c.blue as u8) * SCALE,
    }
}
