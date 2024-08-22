use std::{cmp::max, collections::VecDeque};

use crate::{
    info::*,
    mem::Mmu,
    ppu::tile::{BgMapAttr, OamEntry},
    regs::LcdCtrl,
};

/// One processed pixel with information for constructing its color.
#[derive(Default, Clone, Copy)]
pub(crate) struct Pixel {
    /// 2-bit color index into palette.
    pub(crate) color_id: u8,
    /// In CGB mode it is palette-ID: 0-7.
    /// In non-CGB mode it is palette-ID only for objects: 0-1.
    pub(crate) palette: u8,
    /// Pixel is from an object, use object pallete.
    pub(crate) is_obj: bool,
    /// BG-OBJ priority bit from BGMapAttr, not for object pixels.
    bg_priority: u8,
}

/// Fetch a line of pixels.
/// Put scanned OAM objects in `objects` sorted by OAM index.
/// Use `is_done` to check if line has been constructed and get the
/// pixels from `screen_line`.
#[derive(Default)]
pub(crate) struct LineFetcher {
    /// Objects(sprites) which lie on the current scan line. Max 10.
    /// Objects which come first in OAM should be placed first.
    // For drawing priority following rules are followed:
    // In non-CGB sort by first X-position and then OAM index.
    // In CGB mode sort by OAM index only. In case of a overlap with other
    // objects the one which lies earlier in list this is drawn at the top.
    pub(crate) objects: Vec<OamEntry>,
    /// Containing pixels for the currently being drawn line
    /// It can have some extra pixels.
    pub(crate) screen_line: Vec<Pixel>,

    /// Pixel FIFO, should always contain at least 8-pixels for mixing.
    fifo: VecDeque<Pixel>,
    state: FetcherState,
    // Current scan-position on LCD.
    sx: u8,
    sy: u8,
    /// Window internal line counter.
    win_y: u8,
    /// Fetcher tile X-index.
    fetch_tx: u8,
    /// Cahced LCDC register, updated before each step.
    lcdc: LcdCtrl,

    /// Discard any extra pixels at the start of a line for sub-tile level
    /// scrolling, tile-level scrolling is handeled while tile fetching.
    /// This should be set to `SCX % 8`.
    extra_pixels: u8,

    // Temporary state information.
    /// If window fetching mode, then x-position inside window.
    window: Option<u8>,
    /// If sprite fetching mode, currently being fetched object.
    object: Option<OamEntry>,
    /// Tile info, for all BG/Window and Object.
    tile: TileLine,
}

#[derive(Debug, Default, Clone, Copy)]
enum FetcherState {
    #[default]
    GetTileId,
    GetTileLow,
    GetTileHigh,
    PushPixels,
}

#[derive(Default)]
struct TileLine {
    id: u8,
    low: u8,
    high: u8,
    bank: u8,
    palette: u8,
    line: u8,
    priority: u8,
    xflip: bool,
    yflip: bool,
}

impl LineFetcher {
    pub(crate) fn new() -> Self {
        Self {
            // A FIFO can have maximum of 16-pixels at a time.
            fifo: VecDeque::with_capacity(16),
            state: FetcherState::GetTileId,
            ..Default::default()
        }
    }

    /// Call it once for every 2 dots.
    pub(crate) fn tick_2_dots(&mut self, mmu: &mut Mmu) {
        // We try to emulate the line pixel fetching, drawing and timing
        // as much as possible, but dot timings will not be exact.
        // For each tick call we proceed by two dots.
        // First we see if FIFO has any pixels excess of 8, if so,
        // push that onto LCD line.
        // Then fetch pixels as required.
        // If a window if found while fetching then discard all pixels and start
        // fetch in window mode for the line. Once started a window fetch lasts
        // for the entire line as window extends to right border always.
        // If sprite if detected then fetch it that and mix it with current
        // bg/window pixels in the fifo.

        self.lcdc = LcdCtrl::new(mmu.read(IO_LCDC));
        self.extend_line(mmu);

        use FetcherState::*;
        self.state = match self.state {
            GetTileId if self.object.is_some() => self.fetch_tile_id_obj(mmu),
            GetTileId => self.fetch_tile_id(mmu),
            GetTileLow => self.fetch_tile_low(),
            GetTileHigh => self.fetch_tile_high(mmu),
            PushPixels if self.object.is_some() => self.push_pixels_obj(mmu),
            PushPixels => self.push_pixels(mmu),
        };
    }

    /// Reset and initialize for fetching pixels for a new line.
    /// If Line 0 then, start a new frame.
    /// Call before starting the new line(OAM scan mode).
    pub(crate) fn new_line(&mut self, mmu: &Mmu, line: u8) {
        // Window line counter is incremented only if window was rendered.
        // On line 0 we reset the window internal counter.
        if line == 0 {
            self.win_y = 0;
        } else if self.window.is_some() {
            self.win_y += 1;
        }

        // Clear everything
        self.fifo.clear();
        self.objects.clear();
        self.screen_line.clear();
        self.object = None;
        self.window = None;
        self.state = FetcherState::GetTileId;

        self.sx = 0;
        self.sy = line;
        self.fetch_tx = 0;
        self.extra_pixels = mmu.read(IO_SCX) % 8;

        assert!(self.objects.len() <= MAX_OBJ_PER_LINE);
        if !mmu.is_cgb {
            self.objects.sort_by(|a, b| a.xpos.cmp(&b.xpos));
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.screen_line.len() >= PPU_LINE_PIXELS as usize
    }

    // Fetcher steps for fetching tiles, each take two dots.
    // --------------------------------------------------------------
    fn fetch_tile_id(&mut self, mmu: &Mmu) -> FetcherState {
        let scx = mmu.read(IO_SCX);
        let scy = mmu.read(IO_SCY);
        let tile_map = self.get_tile_map_num();

        // Position within the 256x256 [32x32 tiled] background/window.
        let (tx, y) = if let Some(x) = self.window {
            (x / 8, self.win_y)
        } else {
            ((scx / 8 + self.fetch_tx) % 32, scy.wrapping_add(self.sy))
        };

        self.tile = read_tile_info(mmu, tile_map, tx, y / 8);
        self.tile.line = y % 8;
        FetcherState::GetTileLow
    }

    fn fetch_tile_id_obj(&mut self, mmu: &Mmu) -> FetcherState {
        let obj = self.object.unwrap();
        self.tile = tile_info_from_obj(mmu.is_cgb, obj);

        // Tall objects are comprised of two consecutive tiles.
        // Upper part has even numbered tile-ID.
        // When yflip is enabled the two tiles switch positions.
        if self.lcdc.obj_size == 1 {
            let is_second = self.sy + 16 - obj.ypos > 8;
            self.tile.id = if is_second == self.tile.yflip {
                self.tile.id & !1
            } else {
                self.tile.id | 1
            }
        }
        // Get distance of the scan-line from object's top line.
        // This also works for tall objects: 8x16.
        self.tile.line = (self.sy % 8).wrapping_sub(obj.ypos % 8) % 8;

        FetcherState::GetTileLow
    }

    fn fetch_tile_low(&mut self) -> FetcherState {
        // All data is read in the next step.
        FetcherState::GetTileHigh
    }

    fn fetch_tile_high(&mut self, mmu: &Mmu) -> FetcherState {
        let addr_mode = if self.object.is_some() {
            1 // Objects always follow 1 addressing-mode.
        } else {
            self.lcdc.bg_win_tile_data
        };

        (self.tile.low, self.tile.high) = read_tile_line(
            mmu,
            addr_mode,
            self.tile.bank,
            self.tile.id,
            self.tile.line,
            self.tile.yflip,
            self.tile.xflip,
        );

        FetcherState::PushPixels
    }

    fn push_pixels(&mut self, mmu: &Mmu) -> FetcherState {
        // We push 8-pixels(one tile-line) at once. And FIFO can hold only
        // 16-pixels at a time Therefore, push only if space exits, else wait.
        if self.fifo.len() > 8 {
            return FetcherState::PushPixels;
        }

        // In non-CGB mode lcdc 0-bit controls bg/window enable.
        // If diabled display blank color, that is 0.
        for i in 0..8 {
            let color = if !mmu.is_cgb && self.lcdc.bg_win_priotity == 0 {
                0
            } else {
                tile_color_id(self.tile.low, self.tile.high, i)
            };

            self.fifo.push_back(Pixel {
                color_id: color,
                palette: self.tile.palette,
                bg_priority: self.tile.priority,
                is_obj: false,
            });
        }

        self.fetch_tx += 1;
        self.window = self.window.map(|pos| pos + 8);
        FetcherState::GetTileId
    }

    fn push_pixels_obj(&mut self, mmu: &Mmu) -> FetcherState {
        assert!(self.fifo.len() >= 8);
        let obj = self.object.unwrap();

        // Clip parts of the which are off-screen to the left.
        // Note that obj.xpos is object's X-position + 8.
        let xclip = if obj.xpos < 8 { 8 - obj.xpos } else { 0 };
        for x in xclip..8 {
            let old_idx = (x - xclip) as usize;
            let px = self.mix_obj_pixel(mmu.is_cgb, self.fifo[old_idx], x);
            self.fifo[old_idx] = px;
        }

        // Return to normal operation after processing object pixels.
        self.object = None;
        FetcherState::GetTileId
    }

    fn extend_line(&mut self, mmu: &mut Mmu) {
        if self.fifo.len() <= 8 {
            return;
        }

        if self.extra_pixels > 0 {
            // Pixels must be discarded before drawing anything.
            assert!(self.sx == 0);

            while self.extra_pixels > 0 {
                self.fifo.pop_front();
                self.extra_pixels -= 1;
            }
            return;
        }

        // Try popping 2-pixels as we have 2-dots each step.
        self.pop_pixel_checked(mmu);
        self.pop_pixel_checked(mmu);
    }

    /// Pop and pixel and sent it to LCD if FIFO has more than 8 pixels.
    /// If a window is detected then, discard FIFO pixels and do setup
    /// to start fetching window pixels.
    /// If an object is detected then do setup to fetch its pixels and
    /// do not pop any pixels until the object has been fully processed.
    fn pop_pixel_checked(&mut self, mmu: &Mmu) {
        if self.fifo.len() <= 8 || self.object.is_some() {
            return;
        }

        // If window detected then discard fetched BG-pixel
        // and start fetching window tiles.
        if self.window.is_none() && self.lcdc.win_enable == 1 {
            let wx = mmu.read(IO_WX);
            let wy = mmu.read(IO_WY);

            // Windows top-left position is (wx=7, wy=0).
            if wx <= self.sx + 7 && wy <= self.sy {
                // WX being less than 7 causes abnormal behaviour,
                // so we just clamp it and get real x postion for window.
                self.window = Some(self.sx - (max(7, wx) - 7));
                self.fifo.clear();
                self.extra_pixels = 0;
                return;
            }
        }

        if self.object.is_none() && self.lcdc.obj_enable == 1 {
            self.object = self.get_obj_at(self.sx);

            // If any object at current position then restart the fetch cycle
            // and fetch the object tile-line and attributes.
            if self.object.is_some() {
                assert!(self.fifo.len() >= 8);
                self.state = FetcherState::GetTileId;
                return;
            }
        }

        self.screen_line.push(self.fifo.pop_front().unwrap());
        self.sx += 1;
    }

    /// Get which tile-map to use for BG/Window.
    fn get_tile_map_num(&self) -> u8 {
        if self.window.is_some() {
            if self.lcdc.win_tile_map == 1 {
                1
            } else {
                0
            }
        } else if self.lcdc.bg_tile_map == 1 {
            1
        } else {
            0
        }
    }

    /// Pop off the highest priority object present at x-position.
    fn get_obj_at(&mut self, x: u8) -> Option<OamEntry> {
        for i in 0..self.objects.len() {
            let obj = self.objects[i];
            if obj.xpos <= x + 8 && x + 8 < obj.xpos + 8 {
                return Some(self.objects.remove(i));
            }
        }

        None
    }

    /// Mix old pixels with the current object pixels as per priority.
    /// `obj_px_idx` is object's pixel index in 0-7.
    fn mix_obj_pixel(&self, is_cgb: bool, old: Pixel, obj_px_idx: u8) -> Pixel {
        let obj = self.object.unwrap();

        let (l, h) = (self.tile.low, self.tile.high);
        let px = Pixel {
            palette: self.tile.palette,
            color_id: tile_color_id(l, h, obj_px_idx),
            bg_priority: 0,
            is_obj: true,
        };

        // Color 0 for objects is transparent.
        if px.color_id != 0 && is_obj_priority(is_cgb, self.lcdc, old, obj) {
            px
        } else {
            old
        }
    }
}

/// Returns true if object has priority over BG/Window per BG-OBJ priority.
fn is_obj_priority(is_cgb: bool, lcdc: LcdCtrl, old: Pixel, obj: OamEntry) -> bool {
    // Higher priority objects are drawn first, do not overlap with them.
    if old.is_obj {
        return false;
    }
    // BG color 0 never overlaps with objects.
    if old.color_id == 0 {
        return true;
    }
    // In non-CGB mode for BG colors 1-3 this alone decides priority.
    if !is_cgb {
        return obj.attrs.bg_priority == 0;
    }
    // In CGB mode several bits decide it.
    lcdc.bg_win_priotity == 0 || (old.bg_priority == 0 && obj.attrs.bg_priority == 0)
}

/// Read a line of tile data.
fn read_tile_line(
    mmu: &Mmu,
    addr_mode: u8,
    bank: u8,
    id: u8,
    yoffset: u8,
    yflip: bool,
    xflip: bool,
) -> (u8, u8) {
    let yoff = if yflip {
        7 - yoffset as usize
    } else {
        yoffset as usize
    };

    let addr = tile_data_vram_addr(addr_mode, id);
    let (l, h) = (
        mmu.vram[bank as usize][addr + 2 * yoff],
        mmu.vram[bank as usize][addr + 2 * yoff + 1],
    );

    if xflip {
        (l.reverse_bits(), h.reverse_bits())
    } else {
        (l, h)
    }
}

/// Read tile infomation from given tile-position and map number.
fn read_tile_info(mmu: &Mmu, tile_map: u8, tx: u8, ty: u8) -> TileLine {
    // Tile map is in Bank 0 VRAM and attributes in Bank 1 of VRAM.
    let addr = tile_id_vram_addr(tile_map, tx, ty);
    let id = mmu.vram[0][addr];
    // If in non-CGB mode disable attributes to emulate the same.
    let attrs = BgMapAttr::new(if mmu.is_cgb { mmu.vram[1][addr] } else { 0 });

    TileLine {
        id,
        bank: attrs.bank,
        xflip: attrs.xflip == 1,
        yflip: attrs.yflip == 1,
        priority: attrs.priority,
        ..Default::default()
    }
}

/// Make tile info from an object's `OamEntry`.
fn tile_info_from_obj(is_cgb: bool, obj: OamEntry) -> TileLine {
    let (palette, bank) = if is_cgb {
        (obj.attrs.cgb_palette, obj.attrs.bank)
    } else {
        (obj.attrs.dmg_palette, 0)
    };

    TileLine {
        id: obj.tile_id,
        bank,
        palette,
        priority: obj.attrs.bg_priority,
        xflip: obj.attrs.xflip == 1,
        yflip: obj.attrs.yflip == 1,
        ..Default::default()
    }
}

#[inline]
fn tile_data_vram_addr(addr_mode: u8, tile_id: u8) -> usize {
    // In addr-mode 0, tile is read as: TILE_BLOCK2 + signed_id.
    // In addr-mode 1, tile is read as: TILE_BLOCK0 + unsigned_id.
    let base = (tile_id as usize) * TILE_SIZE;
    let block = match addr_mode {
        1 => TILE_BLOCK0,
        0 => {
            if tile_id < 127 {
                TILE_BLOCK2
            } else {
                TILE_BLOCK1
            }
        }
        _ => panic!("invalid tile addressing mode"),
    };
    base + block - *ADDR_VRAM.start()
}

#[inline]
fn tile_id_vram_addr(tile_map: u8, tx: u8, ty: u8) -> usize {
    let base = match tile_map {
        0 => TILE_MAP0,
        1 => TILE_MAP1,
        _ => unreachable!(),
    };

    // Each tile-map is a 32x32 grid containing 1-byte tiles-IDs.
    base - *ADDR_VRAM.start() + ty as usize * 32 + tx as usize
}

#[inline(always)]
fn tile_color_id(low: u8, hi: u8, x_index: u8) -> u8 {
    debug_assert!(x_index < 8);
    let i = 7 - x_index; // Bit-7 is leftmost pixel.
    ((low >> i) & 1) | ((hi >> i) & 1) << 1
}
