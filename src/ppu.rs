pub(crate) mod fetcher;
mod palettes;

use fetcher::{LineFetcher, OamEntry, Pixel};

use crate::{
    info::*,
    msg::{Color, VideoFrame},
    regs::{CgbColor, IntrBits, LcdStat},
};

// TODO Implement CGB mode rendering and fix issues related to it.

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

    /// ID for mapping monochrome DMG colors to RGB colors.
    dmg_palette_id: usize,
    /// Current PPU mode updates to it are carried to STAT register.
    mode: PpuMode,
    /// Frame containing the screen pixels with double bufferering.
    // Double buffer is required for avoiding choppiness/tearing while drawing.
    frame: [VideoFrame; 2],
    frame_idx: usize,
    /// Amount of dots left, which determines how much to advance.
    dots_left: u32,
    /// Number of dots consumed for the current scan-line `LY`.
    dots_in_line: u32,
    /// STAT interrupt is triggered when this goes from low to high.
    stat_intr_line: bool,
}

const PPU_DRAW_LINES: u32 = SCREEN_RESOLUTION.1 as u32;
const PPU_HSCAN_DOTS: u32 = 456;
const PPU_VBLANK_LINES: u32 = 10;

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
    pub(crate) fn new(is_cgb: bool) -> Self {
        Self {
            fetcher: LineFetcher::new(is_cgb),
            oam: [0; SIZE_OAM],
            bg_palette: [0; SIZE_CGB_PALETTE],
            obj_palette: [0; SIZE_CGB_PALETTE],
            stat: Default::default(),
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,

            dmg_palette_id: palettes::DEFAULT_MONOCHROME,
            mode: PpuMode::Scan,
            frame: Default::default(),
            frame_idx: 0,
            dots_in_line: 0,
            dots_left: 0,
            stat_intr_line: false,
        }
    }

    /// Tick for `dots` cycles, `dots` must be an even number.
    /// Ticks at normal speed even in dual-speed mode.
    pub(crate) fn tick(&mut self, dots: u32) -> IntrBits {
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

    pub(crate) fn cycle_palette(&mut self, direction: i8) {
        assert!(direction != 0, "direction must be either +ve or -ve");

        self.dmg_palette_id = self
            .dmg_palette_id
            .wrapping_add_signed(direction.signum() as isize)
            % palettes::DMG_PALETTES.len();
    }

    pub(crate) fn copy_frame(&self, frame: &mut VideoFrame) {
        *frame = self.frame[1 - self.frame_idx].clone();
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
        self.eat_dots(2);

        match idx {
            0 => self.fetcher.new_line(self.ly),
            OAM_ENTRIES => return PpuMode::Draw,
            _ => return PpuMode::Scan,
        }

        // If the sprite buffer is not full, then a sprite is added to it if:
        // It is on the scan-line as per its Y-pos and objects are enabled.
        // "Ypos - 16" is sprite top position on screen.
        // A sprite can have size: 8x8 or 8x16(tall object mode).
        for i in 0..OAM_ENTRIES {
            let obj = get_oam_entry(&self.oam, i);

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

                self.frame[self.frame_idx].set(i, self.ly as usize, color);
            }

            PpuMode::HBlank
        } else {
            PpuMode::Draw
        }
    }

    fn step_hblank(&mut self) -> PpuMode {
        // If current scan-line finishes and it was last draw line then
        // goto VBlank and swap frame drawing buffers.
        if self.eat_dots(self.dots_left) {
            if self.ly as u32 == PPU_DRAW_LINES {
                self.frame_idx = 1 - self.frame_idx;
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

        if self.ly as u32 == PPU_DRAW_LINES + PPU_VBLANK_LINES {
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

        // On entring VBLANK mode interrupt.
        if new_mode != self.mode && new_mode == PpuMode::VBlank {
            iflag.vblank = 1;
        }

        // IF STAT interrupt source line goes from low-to-high then interrupt.
        let new = calc_stat_interrupt(self.stat, self.mode, self.lyc, self.ly);
        if !self.stat_intr_line && new {
            iflag.stat = 1;
        }
        self.stat_intr_line = new;

        self.stat.ppu_mode = new_mode as u8;
        self.stat.ly_eq_lyc = (self.lyc == self.ly) as u8;
        self.mode = new_mode;
        iflag
    }

    /// Consume as much dots as possible from `dots_left` without overflowing
    /// into the next scan-line. Return true if current scan-line finished.
    fn eat_dots(&mut self, dots: u32) -> bool {
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

    // Pixel to color synthesis
    //---------------------------------------------------------------
    fn pixel_to_color(&self, px: Pixel) -> Color {
        if self.fetcher.is_cgb {
            let color = self.get_cgb_color(px.is_obj, px.color_id, px.palette);
            cgb_to_color(color)
        } else {
            let dmg_palette = &palettes::DMG_PALETTES[self.dmg_palette_id];
            let (color_map, palette) = match (px.is_obj, px.palette) {
                (true, 0) => (dmg_palette.obj0, self.obp0),
                (true, 1) => (dmg_palette.obj1, self.obp1),
                (false, 0) => (dmg_palette.bg, self.bgp),
                _ => unreachable!(),
            };

            // Monochrome palettes stores colors for colors-ID(xx) as:
            // `[MSB] 33-22-11-00 [LSB]`
            let color = (palette >> (px.color_id * 2)) & 0b11;
            color_map[color as usize]
        }
    }

    #[inline]
    fn get_cgb_color(&self, is_obj: bool, color_id: u8, palette_index: u8) -> u16 {
        // Each CGB-palette is of 8-bytes consisting of 4 colors of 2-bytes each.
        let idx = (palette_index * 8 + color_id * 2) as usize;

        u16::from_le_bytes(if is_obj {
            // Transparent object pixels have already filtered by the fetcher.
            [self.obj_palette[idx], self.obj_palette[idx + 1]]
        } else {
            [self.bg_palette[idx], self.bg_palette[idx + 1]]
        })
    }
}

fn calc_stat_interrupt(stat: LcdStat, mode: PpuMode, lyc: u8, ly: u8) -> bool {
    // Logically OR all STAT interrupt sources.
    let ret = stat.lyc_int == 1 && lyc == ly;
    match mode {
        PpuMode::HBlank if stat.mode0 == 1 => true,
        PpuMode::VBlank if stat.mode1 == 1 => true,
        PpuMode::Scan if stat.mode2 == 1 => true,
        _ => ret,
    }
}

#[inline]
fn get_oam_entry(oam: &[u8], idx: usize) -> OamEntry {
    let d = &oam[(idx * 4)..(idx * 4 + 4)];
    OamEntry::from_array([d[0], d[1], d[2], d[3]])
}

#[inline]
fn cgb_to_color(cgb_color: u16) -> Color {
    // Each CGB color component of 5 bits.
    let scale = |x| ((x << 3) | (x >> 2)) as u8;

    let c = CgbColor::new(cgb_color);
    Color {
        r: scale(c.red),
        g: scale(c.green),
        b: scale(c.blue),
    }
}
