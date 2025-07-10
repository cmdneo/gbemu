use std::{cmp::max, collections::VecDeque};

use crate::{info::*, macros::bit_fields, regs::LcdCtrl};

type VramArray = [[u8; SIZE_VRAM_BANK]; VRAM_BANKS];

/// Fetch a line of pixels.
/// Put scanned OAM objects in `objects` sorted by OAM index.
/// Use `is_done` to check if line has been constructed and get the
/// pixels from `screen_line`.
pub(crate) struct LineFetcher {
    pub(super) is_cgb: bool,
    /// Objects(sprites) which lie on the current scan line. Max 10.
    /// Objects which come first in OAM should be placed first.
    pub(crate) objects: Vec<OamEntry>,
    /// Containing pixels for the currently being drawn line.
    pub(crate) screen_line: [Pixel; SCREEN_RESOLUTION.0],

    // Registers and memory owned by it.
    pub(crate) vram: VramArray,
    pub(crate) lcdc: LcdCtrl,
    pub(crate) scx: u8,
    pub(crate) scy: u8,
    pub(crate) wx: u8,
    pub(crate) wy: u8,

    state: FetcherState,
    /// All object pixels are pre-drawn inside this.
    obj_line: [Option<Pixel>; SCREEN_RESOLUTION.0],
    /// Pixel FIFO, it should always contain at least 8-pixels for mixing.
    fifo: VecDeque<Pixel>,
    /// Current draw position on LCD.
    draw_x: u8,
    /// Current position for tile-fetch on BG/Window.
    fetch_x: u8,
    /// Current draw and fetch line, same as to LY when not in VSYNC.
    line: u8,
    /// Window internal line counter.
    win_y: u8,
    /// Discard any extra pixels at the start of a line for sub-tile level
    /// scrolling, tile-level scrolling is handeled while tile fetching.
    /// This should be set to `SCX % 8`.
    subtile_scroll: u8,

    // Temporary state information.
    /// If window fetching mode, then put a window.
    window: Option<()>,
    /// Cached tile info for BG/Window.
    bg_tile: TileLine,
}

#[derive(Default)]
enum FetcherState {
    #[default]
    GetTileId,
    GetTileLow,
    GetTileHigh,
    PushPixels,
}

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
    /// BG-OBJ priority bit from BGMapAttr or OAM attribute.
    bg_priority: u8,
}

// Representation:
// Byte-0: Y-position, Byte-1: X-posiiton, Byte-2: Tile-index
// Byte-3: See OamAttrs.
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct OamEntry {
    /// Object vertical position on screen + 16.
    pub(crate) ypos: u8,
    /// Object horizontal position on screen + 8.
    pub(crate) xpos: u8,
    /// Tile ID
    pub(crate) tile_id: u8,
    /// Object flags and attributes
    attrs: OamAttrs,
}

impl OamEntry {
    pub(crate) fn from_array(a: [u8; 4]) -> Self {
        Self {
            ypos: a[0],
            xpos: a[1],
            tile_id: a[2],
            attrs: OamAttrs::new(a[3]),
        }
    }
}

impl LineFetcher {
    pub(crate) fn new(is_cgb: bool) -> Self {
        Self {
            is_cgb,
            objects: Vec::with_capacity(10),
            screen_line: [Default::default(); SCREEN_RESOLUTION.0],

            vram: [[0; SIZE_VRAM_BANK]; VRAM_BANKS],
            scx: 0,
            scy: 0,
            wx: 0,
            wy: 0,

            state: FetcherState::GetTileId,
            draw_x: 0,
            line: 0,
            obj_line: [Default::default(); SCREEN_RESOLUTION.0],
            fifo: VecDeque::with_capacity(16),
            win_y: 0,
            fetch_x: 0,
            lcdc: Default::default(),
            subtile_scroll: 0,
            window: None,
            bg_tile: Default::default(),
        }
    }

    /// Call it once for every 2 dots.
    pub(crate) fn tick_2_dots(&mut self) {
        // We try to emulate the line pixel fetching, drawing and timing
        // to some extent but dot timings will not be exact.
        // For each tick call we proceed by two dots.
        // First we see if FIFO has any pixels excess of 8, if so,
        // push that to LCD line. Then fetch pixels as required.
        // If a window if found while fetching then discard all pixels and start
        // fetch in window mode for the line. Once started a window fetch lasts
        // for the entire line as window extends to the end of the right border.
        // Objects are drawn in advance in a seperate buffer and mixed with current
        // bg/window pixels in the fifo as per bg-win priority bits.
        // TODO Emulate object fetching to get more accurate timings.

        if !self.objects.is_empty() {
            self.render_oam_objects();
        }
        self.push_pixels_to_line();

        self.state = match self.state {
            FetcherState::GetTileId => FetcherState::GetTileLow,
            FetcherState::GetTileLow => FetcherState::GetTileHigh,
            FetcherState::GetTileHigh => self.fetch_tile(),
            FetcherState::PushPixels => self.push_pixels(),
        };
    }

    /// Initialize for fetching pixels for a new line and set LY.
    /// If Line 0 then, start a new frame.
    /// Call before starting a new line(OAM scan mode).
    pub(crate) fn new_line(&mut self, line: u8) {
        // Window line counter is incremented only if window was rendered.
        // On line 0 we reset the window internal counter.
        if line == 0 {
            self.win_y = 0;
        } else if self.window.is_some() {
            self.win_y += 1;
        }

        // Clear and set everything
        self.fifo.clear();
        self.objects.clear();
        self.screen_line.fill_with(Pixel::default);
        self.obj_line.fill_with(|| None);
        self.window = None;
        self.fetch_x = 0;
        self.draw_x = 0;
        self.line = line;
        self.subtile_scroll = self.scx % 8;
        self.state = FetcherState::GetTileId;
    }

    pub(crate) fn is_done(&self) -> bool {
        self.draw_x as usize == SCREEN_RESOLUTION.0
    }

    // Fetcher steps for fetching tiles, each take two dots.
    // --------------------------------------------------------------
    fn fetch_tile(&mut self) -> FetcherState {
        let tile_map = self.get_tile_map_number();

        // Position within the 256x256 px [32x32 tiled] background/window.
        let (tx, y) = if self.window.is_some() {
            (self.fetch_x / 8, self.win_y)
        } else {
            (
                (self.scx / 8 + self.fetch_x / 8) % 32,
                self.scy.wrapping_add(self.line),
            )
        };

        self.bg_tile = TileLine::from_tilemap(&self.vram, tile_map, tx, y / 8, self.is_cgb);
        self.bg_tile.line = y % 8;
        self.bg_tile
            .read_in_line(&self.vram, self.lcdc.bg_win_tile_data);

        FetcherState::PushPixels
    }

    fn push_pixels(&mut self) -> FetcherState {
        // We push 8-pixels(one tile-line) at once. And FIFO can hold only
        // 16-pixels at a time Therefore, push only if space exits, else wait.
        if self.fifo.len() > 8 {
            return FetcherState::PushPixels;
        }

        // In non-CGB mode lcdc 0-bit controls bg/window enable.
        // If diabled its displays the blank color, that is 0.
        for i in 0..8 {
            let color = if !self.is_cgb && self.lcdc.bg_win_priotity == 0 {
                0
            } else {
                self.bg_tile.get_color_id(i)
            };

            self.fifo.push_back(Pixel {
                color_id: color,
                palette: self.bg_tile.palette,
                bg_priority: self.bg_tile.bg_priority,
                is_obj: false,
            });
        }

        self.fetch_x += 8;

        FetcherState::GetTileId
    }

    /// Push any pixels excess of 8 to screen line.
    fn push_pixels_to_line(&mut self) {
        if self.fifo.len() <= 8 {
            return;
        }

        if self.subtile_scroll > 0 {
            assert!(self.draw_x == 0);
            for _ in 0..self.subtile_scroll {
                self.fifo.pop_front();
            }

            self.subtile_scroll = 0;
            return;
        }

        // Try popping 2-pixels as we have 2-dots for each step.
        self.pop_pixel_checked();
        self.pop_pixel_checked();
    }

    /// Pop and pixel and send it to the LCD if FIFO has more than 8 pixels.
    /// If a window is detected then setup fetcher to start fetching
    /// their pixels for drawing.
    fn pop_pixel_checked(&mut self) {
        if self.fifo.len() <= 8 {
            return;
        }
        if self.draw_x as usize == SCREEN_RESOLUTION.0 {
            return;
        }

        // If window detected then discard fetched BG-pixel
        // and start fetching window tiles for this line.
        if self.window.is_none() && self.lcdc.win_enable == 1 {
            // Windows top-left position is (wx=7, wy=0).
            if self.wx <= self.draw_x + 7 && self.wy <= self.line {
                // WX being less than 7 causes abnormal behaviour, so we just
                // clamp it and get x-position for fetching the window.
                self.fetch_x = self.draw_x - (max(7, self.wx) - 7);
                self.window = Some(());
                self.fifo.clear();
                return;
            }
        }

        let bg_px = self.fifo.pop_front().unwrap();
        let obj_px = self.obj_line[self.draw_x as usize];

        // Mix BG/Win pixel with object pixel(if present and enabled).
        let px = match obj_px {
            Some(obj_px) if self.lcdc.obj_enable == 1 => self.mix_bg_obj_pixels(bg_px, obj_px),
            _ => bg_px,
        };

        self.screen_line[self.draw_x as usize] = px;
        self.draw_x += 1;
    }

    /// Remove each object from `objects` and draw it.
    fn render_oam_objects(&mut self) {
        assert!(self.objects.len() <= MAX_OBJ_PER_LINE);

        // For object drawing priority, higher priority comes first:
        // In non-CGB mode sort using (X-position, OAM-index).
        // In CGB mode sort using (OAM-index) only.
        // In case of overlap higher priority objects are placed on top.
        if !self.is_cgb {
            self.objects.sort_by(|a, b| a.xpos.cmp(&b.xpos));
        }
        self.objects.reverse(); // We draw by popping from end, so reverse it.

        while let Some(obj) = self.objects.pop() {
            self.render_object(obj);
        }
    }

    fn render_object(&mut self, obj: OamEntry) {
        // The obj.xpos stores object's X-position + 8. So,
        // clip parts of the object which are off-screen to the left.
        let tile = self.read_obj_tile_line(obj);
        let xclip = 8u8.saturating_sub(obj.xpos); // if obj.xpos < 8 { 8 - obj.xpos } else { 0 };
        let xbegin = obj.xpos.saturating_sub(8);
        let xend = obj.xpos.min(SCREEN_RESOLUTION.0 as u8);

        for x in xbegin..xend {
            let px = Pixel {
                palette: tile.palette,
                color_id: tile.get_color_id(x - xbegin + xclip),
                is_obj: true,
                bg_priority: obj.attrs.bg_priority,
            };

            // Color 0 of object is transparent so we never add it.
            // And do not draw over already drawn(higher priority) objects.
            if self.obj_line[x as usize].is_none() && px.color_id != 0 {
                self.obj_line[x as usize] = Some(px);
            }
        }
    }

    fn read_obj_tile_line(&mut self, obj: OamEntry) -> TileLine {
        let mut ret = TileLine::from_obj(obj, self.is_cgb);

        // Tall objects are comprised of two consecutive tiles.
        // Upper part has even numbered tile-ID.
        if self.lcdc.obj_size == 1 {
            let is_second = self.line + 16 >= 8 + obj.ypos;

            // When yflip is enabled the two tiles switch positions.
            if is_second == ret.yflip {
                ret.id &= !1; // Even
            } else {
                ret.id |= 1; // Odd
            }
        }

        ret.line = self.line.wrapping_sub(obj.ypos) % 8;
        ret.read_in_line(&self.vram, 1); // Objects always use addr-mode 1.

        ret
    }

    /// Get which tile-map to use for BG/Window.
    #[inline]
    fn get_tile_map_number(&self) -> u8 {
        if self.window.is_some() {
            self.lcdc.win_tile_map
        } else {
            self.lcdc.bg_tile_map
        }
    }

    #[inline]
    fn mix_bg_obj_pixels(&self, bg: Pixel, obj: Pixel) -> Pixel {
        if obj.color_id == 0 {
            return bg; // Obj color 0 is transparent.
        }
        if bg.color_id == 0 {
            return obj; // Obj always has priority over color 0 of BG/Win.
        }

        if self.is_cgb {
            if self.lcdc.bg_win_priotity & (bg.bg_priority | obj.bg_priority) == 1 {
                return bg;
            }
        } else if obj.bg_priority == 1 {
            return bg;
        }

        obj
    }
}

bit_fields! {
    /// OAM attribute.
    #[derive(Debug)]
    struct OamAttrs<u8> {
        cgb_palette: 3,
        bank: 1,
        dmg_palette:1,
        xflip:1,
        yflip:1,
        bg_priority:1,
    }
}

bit_fields! {
    /// In CGB mode VRAM Bank-1 stores a seperate 32x32 bytes attribute map,
    /// where, each byte stores attributes for the corresponding tile-number
    /// map entry present in VRAM Bank 0.
    ///
    /// BG map attributes, for CGB mode only.
    struct BgMapAttr<u8> {
        palette: 3,
        bank: 1,
        _0: 1,
        xflip: 1,
        yflip: 1,
        priority: 1,
    }
}

#[derive(Default, Clone, Copy)]
struct TileLine {
    id: u8,
    low: u8,
    high: u8,
    bank: u8,
    palette: u8,
    line: u8,
    bg_priority: u8,
    xflip: bool,
    yflip: bool,
}

impl TileLine {
    fn from_obj(obj: OamEntry, is_cgb: bool) -> Self {
        let (palette, bank) = if is_cgb {
            (obj.attrs.cgb_palette, obj.attrs.bank)
        } else {
            (obj.attrs.dmg_palette, 0)
        };

        Self {
            id: obj.tile_id,
            bank,
            palette,
            bg_priority: obj.attrs.bg_priority,
            xflip: obj.attrs.xflip == 1,
            yflip: obj.attrs.yflip == 1,
            ..Default::default()
        }
    }

    fn from_tilemap(vram: &VramArray, tile_map: u8, tx: u8, ty: u8, is_cgb: bool) -> Self {
        // Tile map is in Bank-0 and attributes in Bank-1 of VRAM.
        let addr = tile_id_vram_addr(tile_map, tx, ty);
        let id = vram[0][addr];
        // If in non-CGB mode disable attributes to emulate the same.
        let attrs = BgMapAttr::new(if is_cgb { vram[1][addr] } else { 0 });

        Self {
            id,
            bank: attrs.bank,
            xflip: attrs.xflip == 1,
            yflip: attrs.yflip == 1,
            bg_priority: attrs.priority,
            ..Default::default()
        }
    }

    /// Read a line of tile data.
    fn read_in_line(&mut self, vram: &VramArray, addr_mode: u8) {
        let yoffset = if self.yflip { 7 - self.line } else { self.line } as usize;

        let addr = tile_data_vram_addr(addr_mode, self.id);
        let (l, h) = (
            vram[self.bank as usize][addr + 2 * yoffset],
            vram[self.bank as usize][addr + 2 * yoffset + 1],
        );

        (self.low, self.high) = if self.xflip {
            (l.reverse_bits(), h.reverse_bits())
        } else {
            (l, h)
        };
    }

    #[inline]
    fn get_color_id(&self, column: u8) -> u8 {
        debug_assert!(column < 8);

        let i = 7 - column; // MSB is the leftmost pixel.
        ((self.low >> i) & 1) | ((self.high >> i) & 1) << 1
    }
}

fn tile_data_vram_addr(addr_mode: u8, tile_id: u8) -> usize {
    // In addr-mode 0, tile is read as: TILE_BLOCK2 + signed_offset * stride.
    // In addr-mode 1, tile is read as: TILE_BLOCK0 + unsigned_offset * stride.
    let addr = match addr_mode {
        0 => TILE_BLOCK2.wrapping_add((tile_id as i8 as isize as usize).wrapping_mul(TILE_SIZE)),
        1 => TILE_BLOCK0 + (tile_id as usize * TILE_SIZE),
        _ => unreachable!(),
    };

    addr - *ADDR_VRAM.start()
}

#[inline]
fn tile_id_vram_addr(tile_map: u8, tx: u8, ty: u8) -> usize {
    let base = match tile_map {
        0 => TILE_MAP0,
        1 => TILE_MAP1,
        _ => unreachable!(),
    };

    // Each tile-map is a 32x32 grid containing 1-byte tiles-IDs.
    let offset = (ty as usize) * 32 + (tx as usize);
    base + offset - *ADDR_VRAM.start()
}
