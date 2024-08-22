use crate::macros::bit_fields;

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
    pub(crate) attrs: OamAttrs,
}

bit_fields! {
    /// OAM attribute. Can be used as a generic tile attribute.
    #[derive(Debug)]
    pub(crate) struct OamAttrs<u8> {
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
    pub(crate) struct BgMapAttr<u8> {
        palette: 3,
        bank: 1,
        _0: 1,
        xflip: 1,
        yflip: 1,
        priority: 1,
    }
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
