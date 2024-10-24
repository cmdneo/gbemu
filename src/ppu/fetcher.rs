use std::{cmp::max, collections::VecDeque};

use crate::{info::*, macros::bit_fields, regs::LcdCtrl};

type VramArray = [[u8; SIZE_VRAM_BANK]; VRAM_BANKS];

/// Fetch a line of pixels.
/// Put scanned OAM objects in `objects` sorted by OAM index.
/// Use `is_done` to check if line has been constructed and get the
/// pixels from `screen_line`.
pub(crate) struct LineFetcher {
    /// Objects(sprites) which lie on the current scan line. Max 10.
    /// Objects which come first in OAM should be placed first.
    // For drawing priority following rules are followed:
    // In non-CGB sort by first X-position and then OAM index.
    // In CGB mode sort by OAM index only. In case of a overlap with other
    // objects the one which lies earlier in list this is drawn at the top.
    pub(crate) objects: Vec<OamEntry>,
    /// Containing pixels for the currently being drawn line.
    pub(crate) screen_line: Vec<Pixel>,
    pub(crate) is_2x: bool,

    // Registers and memory owned by it.
    pub(crate) vram: VramArray,
    pub(crate) lcdc: LcdCtrl,
    pub(crate) scx: u8,
    pub(crate) scy: u8,
    pub(crate) wx: u8,
    pub(crate) wy: u8,

    /// Pixel FIFO, it should always contain at least 8-pixels for mixing.
    fifo: VecDeque<Pixel>,
    state: FetcherState,
    /// Current draw position on LCD.
    draw_x: u8,
    /// Current position for tile-fetch on BG/Window.
    fetch_x: u8,
    /// Current draw and fetch line, it is synchronized to LY by the PPU.
    line: u8,
    /// Window internal line counter.
    win_y: u8,
    /// Discard any extra pixels at the start of a line for sub-tile level
    /// scrolling, tile-level scrolling is handeled while tile fetching.
    /// This should be set to `SCX % 8`.
    tile_extra_pixels: u8,
    // Temporary state information.
    /// If window fetching mode, then put a window.
    window: Option<()>,
    /// If sprite fetching mode, currently being fetched object.
    object: Option<OamEntry>,
    /// Tile info, for all BG/Window and Object.
    tile: TileLine,
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

    /// BG-OBJ priority bit from BGMapAttr, not for object pixels.
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
    tile_id: u8,
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
    pub(crate) fn new() -> Self {
        Self {
            is_2x: false,
            fifo: VecDeque::with_capacity(16),
            state: FetcherState::GetTileId,
            objects: Vec::with_capacity(10),
            screen_line: Vec::with_capacity(SCREEN_RESOLUTION.0),
            vram: [[0; SIZE_VRAM_BANK]; VRAM_BANKS],
            scx: 0,
            scy: 0,
            draw_x: 0,
            line: 0,
            wx: 0,
            wy: 0,
            win_y: 0,
            fetch_x: 0,
            lcdc: Default::default(),
            tile_extra_pixels: 0,
            window: None,
            object: None,
            tile: Default::default(),
        }
    }

    /// Call it once for every 2 dots.
    pub(crate) fn tick_2_dots(&mut self) {
        // We try to emulate the line pixel fetching, drawing and timing
        // as much as possible, but dot timings will not be exact.
        // For each tick call we proceed by two dots.
        // First we see if FIFO has any pixels excess of 8, if so,
        // push that to LCD line. Then fetch pixels as required.
        // If a window if found while fetching then discard all pixels and start
        // fetch in window mode for the line. Once started a window fetch lasts
        // for the entire line as window extends to the end of the right border.
        // If sprite if detected then fetch it that and mix it with current
        // bg/window pixels in the fifo as per bg-win priority bits.

        self.push_pixels_to_line();

        self.state = match self.state {
            FetcherState::GetTileId => {
                if self.object.is_some() {
                    self.fetch_tile_id_obj()
                } else {
                    self.fetch_tile_id()
                }
            }
            FetcherState::GetTileLow => self.fetch_tile_low(),
            FetcherState::GetTileHigh => self.fetch_tile_high(),
            FetcherState::PushPixels => {
                if self.object.is_some() {
                    self.push_pixels_obj()
                } else {
                    self.push_pixels()
                }
            }
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

        // Clear and reset everything
        self.fifo.clear();
        self.objects.clear();
        self.screen_line.clear();
        self.object = None;
        self.window = None;
        self.fetch_x = 0;
        self.draw_x = 0;
        self.line = line;
        self.tile_extra_pixels = self.scx % 8;
        self.state = FetcherState::GetTileId;

        assert!(self.objects.len() <= MAX_OBJ_PER_LINE);
        if !self.is_2x {
            self.objects.sort_by(|a, b| a.xpos.cmp(&b.xpos));
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.screen_line.len() >= PPU_LINE_PIXELS as usize
    }

    // Fetcher steps for fetching tiles, each take two dots.
    // --------------------------------------------------------------
    fn fetch_tile_id(&mut self) -> FetcherState {
        let tile_map = self.get_tile_map_num();

        // Position within the 256x256 px [32x32 tiled] background/window.
        let (tx, y) = if self.window.is_some() {
            // TODO Is window tile-X calculation right? Test it.
            (self.fetch_x / 8, self.win_y)
        } else {
            (
                (self.scx / 8 + self.fetch_x / 8) % 32,
                self.scy.wrapping_add(self.line),
            )
        };

        self.tile = read_tile_info(self.is_2x, &self.vram, tile_map, tx, y / 8);
        self.tile.line = y % 8;

        FetcherState::GetTileLow
    }

    fn fetch_tile_id_obj(&mut self) -> FetcherState {
        let obj = self.object.unwrap();
        self.tile = tile_info_from_obj(self.is_2x, obj);

        // Tall objects are comprised of two consecutive tiles.
        // Upper part has even numbered tile-ID.
        // When yflip is enabled the two tiles switch positions.
        if self.lcdc.obj_size == 1 {
            let is_second = self.line + 16 - obj.ypos > 8;
            self.tile.id = if is_second == self.tile.yflip {
                self.tile.id & !1
            } else {
                self.tile.id | 1
            }
        }
        // Get distance of the scan-line from object-tile's top line
        // for selecting which line of the tile will be drawn.
        self.tile.line = (self.line % 8).wrapping_sub(obj.ypos % 8) % 8;

        FetcherState::GetTileLow
    }

    fn fetch_tile_low(&mut self) -> FetcherState {
        // All data is read in the next step.
        FetcherState::GetTileHigh
    }

    fn fetch_tile_high(&mut self) -> FetcherState {
        let addr_mode = if self.object.is_some() {
            1 // Objects always follow 1 addressing-mode.
        } else {
            self.lcdc.bg_win_tile_data
        };

        (self.tile.low, self.tile.high) = read_tile_line(
            &self.vram,
            addr_mode,
            self.tile.bank,
            self.tile.id,
            self.tile.line,
            self.tile.yflip,
            self.tile.xflip,
        );

        FetcherState::PushPixels
    }

    fn push_pixels(&mut self) -> FetcherState {
        // We push 8-pixels(one tile-line) at once. And FIFO can hold only
        // 16-pixels at a time Therefore, push only if space exits, else wait.
        if self.fifo.len() > 8 {
            return FetcherState::PushPixels;
        }

        // In non-CGB mode lcdc 0-bit controls bg/window enable.
        // If diabled display blank color, that is 0.
        for i in 0..8 {
            let color = if !self.is_2x && self.lcdc.bg_win_priotity == 0 {
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

        self.fetch_x += 8;
        FetcherState::GetTileId
    }

    fn push_pixels_obj(&mut self) -> FetcherState {
        assert!(self.fifo.len() >= 8);
        let obj = self.object.unwrap();

        // Clip parts of the which are off-screen to the left.
        // obj.xpos is object's real X-position + 8.
        let xclip_start = if obj.xpos < 8 { 8 - obj.xpos } else { 0 };
        for x in xclip_start..8 {
            let old_idx = (x - xclip_start) as usize;
            let px = self.mix_obj_pixel(self.is_2x, self.fifo[old_idx], x);
            self.fifo[old_idx] = px;
        }

        // Return to normal operation after processing object pixels.
        self.object = None;
        FetcherState::GetTileId
    }

    /// Push any pixels excess of 8 to screen line.
    fn push_pixels_to_line(&mut self) {
        if self.fifo.len() <= 8 {
            return;
        }

        if self.tile_extra_pixels > 0 {
            assert!(self.draw_x == 0);
            for _ in 0..self.tile_extra_pixels {
                self.fifo.pop_front();
            }

            self.tile_extra_pixels = 0;
            return;
        }

        // Try popping 2-pixels as we have 2-dots each step.
        self.pop_pixel_checked();
        self.pop_pixel_checked();
    }

    /// Pop and pixel and sent it to LCD if FIFO has more than 8 pixels.
    /// If a window is detected then, discard FIFO pixels and do setup
    /// to start fetching window pixels.
    /// If an object is detected then do setup to fetch its pixels and
    /// do not pop any pixels until the object has been fully processed.
    fn pop_pixel_checked(&mut self) {
        if self.fifo.len() <= 8 || self.object.is_some() {
            return;
        }

        // If window detected then discard fetched BG-pixel
        // and start fetching window tiles for this line.
        if self.window.is_none() && self.lcdc.win_enable == 1 {
            // Windows top-left position is (wx=7, wy=0).
            if self.wx <= self.draw_x + 7 && self.wy <= self.line {
                // WX being less than 7 causes abnormal behaviour,
                // so we just clamp it and get real x postion for window.
                self.fetch_x = self.draw_x - (max(7, self.wx) - 7);
                self.window = Some(());
                self.fifo.clear();
                return;
            }
        }

        // If any object at current position then restart the fetch cycle
        // and fetch the object tile-line and attributes for pixel mixing.
        if self.object.is_none() && self.lcdc.obj_enable == 1 {
            self.object = self.pop_obj_at(self.draw_x);

            if self.object.is_some() {
                assert!(self.fifo.len() >= 8);
                self.state = FetcherState::GetTileId;
                return;
            }
        }

        self.screen_line.push(self.fifo.pop_front().unwrap());
        self.draw_x += 1;
    }

    /// Pop off and return the highest priority object lying on `xpos`.
    fn pop_obj_at(&mut self, xpos: u8) -> Option<OamEntry> {
        for i in 0..self.objects.len() {
            let obj = self.objects[i];
            if obj.xpos <= xpos + 8 && xpos + 8 < obj.xpos + 8 {
                return Some(self.objects.remove(i));
            }
        }

        None
    }

    /// Get which tile-map to use for BG/Window.
    fn get_tile_map_num(&self) -> u8 {
        if self.window.is_some() {
            self.lcdc.win_tile_map
        } else {
            self.lcdc.bg_tile_map
        }
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

        // FIXME Fix object overlaid over BG/Window wrongly.
        // Color 0 for objects is transparent.
        if px.color_id != 0 && is_obj_priority(is_cgb, self.lcdc, old, obj) {
            px
        } else {
            old
        }
    }
}

bit_fields! {
    /// OAM attribute. Can be used as a generic tile attribute.
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

#[derive(Default)]
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

/// Determines if object pixel has priority over already drawn BG/Window/Object pixel.
fn is_obj_priority(is_cgb: bool, lcdc: LcdCtrl, old: Pixel, obj: OamEntry) -> bool {
    // Higher priority objects pixels are drawn above lower priority objects.
    if old.is_obj {
        return false;
    }
    // BG color 0 never overlaps with objects.
    if old.color_id == 0 {
        return true;
    }
    // In non-CGB mode for BG colors 1-3 this attr bit alone decides priority.
    if !is_cgb {
        return obj.attrs.bg_priority == 0;
    }
    // In CGB mode several bits decide it.
    lcdc.bg_win_priotity == 0 || (old.bg_priority == 0 && obj.attrs.bg_priority == 0)
}

/// Read a line of tile data.
fn read_tile_line(
    vram: &VramArray,
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
        vram[bank as usize][addr + 2 * yoff],
        vram[bank as usize][addr + 2 * yoff + 1],
    );

    if xflip {
        (l.reverse_bits(), h.reverse_bits())
    } else {
        (l, h)
    }
}

/// Read tile infomation from given tile-position and map number.
fn read_tile_info(is_2x: bool, vram: &VramArray, tile_map: u8, tx: u8, ty: u8) -> TileLine {
    // Tile map is in Bank 0 VRAM and attributes in Bank 1 of VRAM.
    let addr = tile_id_vram_addr(tile_map, tx, ty);
    let id = vram[0][addr];
    // If in non-CGB mode disable attributes to emulate the same.
    let attrs = BgMapAttr::new(if is_2x { vram[1][addr] } else { 0 });

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

fn tile_data_vram_addr(addr_mode: u8, tile_id: u8) -> usize {
    // In addr-mode 0, tile is read as: TILE_BLOCK2 + signed_offset.
    // In addr-mode 1, tile is read as: TILE_BLOCK0 + unsigned_offset.
    let addr = match addr_mode {
        0 => TILE_BLOCK2.wrapping_add((tile_id as i8 as isize as usize).wrapping_mul(TILE_SIZE)),
        1 => TILE_BLOCK0 + (tile_id as usize * TILE_SIZE),
        _ => panic!("invalid tile addressing mode"),
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
    let rel = (ty as usize) * 32 + (tx as usize);
    base + rel - *ADDR_VRAM.start()
}

#[inline(always)]
fn tile_color_id(low: u8, hi: u8, column: u8) -> u8 {
    debug_assert!(column < 8);
    let i = 7 - column; // Bit-7 is leftmost pixel.
    ((low >> i) & 1) | ((hi >> i) & 1) << 1
}
