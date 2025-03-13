//! Collection of constants used throughout the emulator.

type URange = std::ops::RangeInclusive<usize>;

/// One kibibyte
pub(crate) const KB: usize = 1 << 10;

// Timing parameters
// --------------------------------------------------------
pub(crate) const FREQUENCY: u32 = 1 << 22; // ~4.19 MHz
pub(crate) const FREQUENCY_2X: u32 = 1 << 23; // ~8.38 Mhz
/// Time for which CPU remains stalled after a speed-switch.
// pub(crate) const SPEED_SWITCH_MCYCLES: u16 = 2050;

// Memory system mapping, address and size information.
// --------------------------------------------------------
// Memory sizes
pub(crate) const SIZE_ROM_BANK: usize = 16 * KB;
pub(crate) const SIZE_VRAM_BANK: usize = 8 * KB;
pub(crate) const SIZE_EXT_RAM: usize = 8 * KB;
pub(crate) const SIZE_WRAM_BANK: usize = 4 * KB;
pub(crate) const SIZE_OAM: usize = 160;
// pub(crate) const SIZE_IO_REGS: usize = 128;
pub(crate) const SIZE_HRAM: usize = 127;
pub(crate) const SIZE_AUDIO_WAVE_RAM: usize = 16;

// Switchable banks count.
pub(crate) const VRAM_BANKS: usize = 2;
pub(crate) const WRAM_BANKS: usize = 8;

// Address mapping ranges
pub(crate) const ADDR_ROM0: URange = 0x0000..=0x3FFF;
pub(crate) const ADDR_ROM1: URange = 0x4000..=0x7FFF;
pub(crate) const ADDR_VRAM: URange = 0x8000..=0x9FFF;
pub(crate) const ADDR_EXT_RAM: URange = 0xA000..=0xBFFF;
pub(crate) const ADDR_WRAM0: URange = 0xC000..=0xCFFF;
pub(crate) const ADDR_WRAM1: URange = 0xD000..=0xDFFF;
pub(crate) const ADDR_ECHO_RAM: URange = 0xE000..=0xFDFF;
pub(crate) const ADDR_OAM: URange = 0xFE00..=0xFE9F;
pub(crate) const ADDR_UNUSABLE: URange = 0xFEA0..=0xFEFF;
pub(crate) const ADDR_IO_REGS: URange = 0xFF00..=0xFF7F;
pub(crate) const ADDR_HRAM: URange = 0xFF80..=0xFFFE;
pub(crate) const ADDR_IE: URange = 0xFFFF..=0xFFFF;

// Only lower 13-bits are connected to the WRAM0 for echo RAM.
pub(crate) const ECHO_RAM_ADDR_MASK: usize = !(!0 << 13);

// VRAM, OAM, PPU and graphics related information.
// --------------------------------------------------------
pub(crate) const SCREEN_RESOLUTION: (usize, usize) = (160, 144);

// Start address for different VRAM tile data and map areas
pub(crate) const TILE_BLOCK0: usize = 0x8000;
// pub(crate) const TILE_BLOCK1: usize = 0x8800;
pub(crate) const TILE_BLOCK2: usize = 0x9000;
pub(crate) const TILE_MAP0: usize = 0x9800;
pub(crate) const TILE_MAP1: usize = 0x9C00;
pub(crate) const TILE_SIZE: usize = 16;

// 8 palettes, each having 4 colors, where each color is 2 bytes.
pub(crate) const SIZE_CGB_PALETTE: usize = 64;

pub(crate) const OAM_ENTRIES: usize = 40;
pub(crate) const MAX_OBJ_PER_LINE: usize = 10;

// PPU modes, stored in IO_STAT register.
pub(crate) const MODE_HBLANK: u8 = 0;
pub(crate) const MODE_VBLANK: u8 = 1;
pub(crate) const MODE_SCAN: u8 = 2;
pub(crate) const MODE_DRAW: u8 = 3;

// IO register addresses.
//---------------------------------------------------------
/// Joypad input
pub(crate) const IO_JOYPAD: usize = 0xFF00;

// Serial transfer
pub(crate) const IO_SB: usize = 0xFF01;
pub(crate) const IO_SC: usize = 0xFF02;

// Timer and divider
pub(crate) const IO_DIV: usize = 0xFF04;
pub(crate) const IO_TIMA: usize = 0xFF05;
pub(crate) const IO_TMA: usize = 0xFF06;
pub(crate) const IO_TAC: usize = 0xFF07;

// Interrupts flag and enable
pub(crate) const IO_IF: usize = 0xFF0F;
pub(crate) const IO_IE: usize = 0xFFFF;

// Audio channel 1
pub(crate) const IO_NR10: usize = 0xFF10;
pub(crate) const IO_NR11: usize = 0xFF11;
pub(crate) const IO_NR12: usize = 0xFF12;
pub(crate) const IO_NR13: usize = 0xFF13;
pub(crate) const IO_NR14: usize = 0xFF14;

// Audio channel 2
pub(crate) const IO_NR21: usize = 0xFF16;
pub(crate) const IO_NR22: usize = 0xFF17;
pub(crate) const IO_NR23: usize = 0xFF18;
pub(crate) const IO_NR24: usize = 0xFF19;

// Audio channel 3
pub(crate) const IO_NR30: usize = 0xFF1A;
pub(crate) const IO_NR31: usize = 0xFF1B;
pub(crate) const IO_NR32: usize = 0xFF1C;
pub(crate) const IO_NR33: usize = 0xFF1D;
pub(crate) const IO_NR34: usize = 0xFF1E;

// Audio channel 4
pub(crate) const IO_NR41: usize = 0xFF20;
pub(crate) const IO_NR42: usize = 0xFF21;
pub(crate) const IO_NR43: usize = 0xFF22;
pub(crate) const IO_NR44: usize = 0xFF23;

// Audio channel 5(global)
pub(crate) const IO_NR50: usize = 0xFF24;
pub(crate) const IO_NR51: usize = 0xFF25;
pub(crate) const IO_NR52: usize = 0xFF26;

// LCD: control, status, position and scrolling
pub(crate) const IO_LCDC: usize = 0xFF40;
pub(crate) const IO_STAT: usize = 0xFF41;
pub(crate) const IO_SCY: usize = 0xFF42;
pub(crate) const IO_SCX: usize = 0xFF43;
pub(crate) const IO_LY: usize = 0xFF44;
pub(crate) const IO_LYC: usize = 0xFF45;
pub(crate) const IO_WY: usize = 0xFF4A;
pub(crate) const IO_WX: usize = 0xFF4B;

// LCD(DMG) monochrome palette
pub(crate) const IO_BGP: usize = 0xFF47;
pub(crate) const IO_OBP0: usize = 0xFF48;
pub(crate) const IO_OBP1: usize = 0xFF49;

// LCD(CGB) color palette and object priority mode
pub(crate) const IO_BGPI: usize = 0xFF68;
pub(crate) const IO_BGPD: usize = 0xFF69;
pub(crate) const IO_OBPI: usize = 0xFF6A;
pub(crate) const IO_OBPD: usize = 0xFF6B;
pub(crate) const IO_OPRI: usize = 0xFF6C;

pub(crate) const ADDR_AUDIO_WAVE_RAM: URange = 0xFF30..=0xFF3F;

/// Select WRAM bank: 1-7.
pub(crate) const IO_SVBK: usize = 0xFF70;

/// VRAM bank select: 0-1.
pub(crate) const IO_VBK: usize = 0xFF4F;

// VRAM DMA: src(1:hi, 2:lo), dst(3:hi, 4:lo) and 5:length/mode/start.
// pub(crate) const IO_HDMA1: usize = 0xFF51;
// pub(crate) const IO_HDMA2: usize = 0xFF52;
// pub(crate) const IO_HDMA3: usize = 0xFF53;
// pub(crate) const IO_HDMA4: usize = 0xFF54;
// pub(crate) const IO_HDMA5: usize = 0xFF55;

/// OAM DMA control
pub(crate) const IO_DMA: usize = 0xFF46;

/// Speed switch for CGB dual-speed mode.
pub(crate) const IO_KEY1: usize = 0xFF4D;

/// IR communications port
pub(crate) const IO_RP: usize = 0xFF56;

// Cartridge header layout information.
// Fields not relevant to the emulator implementation are not listed here.
//---------------------------------------------------------
pub(crate) const CART_HEADER: URange = 0x100..=0x14F;

pub(crate) const CART_ENTRY: URange = 0x100..=0x103;
pub(crate) const CART_LOGO: URange = 0x104..=0x133;
pub(crate) const CART_TITLE: URange = 0x134..=0x143;
pub(crate) const CART_CGB_FLAG: usize = 0x143;
pub(crate) const CART_SGB_FLAG: usize = 0x146;
pub(crate) const CART_TYPE: usize = 0x147;
pub(crate) const CART_RAM_SIZE: usize = 0x149;
pub(crate) const CART_HEADER_CSUM: usize = 0x14D;
pub(crate) const CART_GLOBAL_CSUM: URange = 0x14E..=0x14F;

/// In real gameboys the value of logo in header should be equal to
/// this value, otherwise, the game will not run on real hardware.
pub(crate) const CART_LOGO_VAL: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

/// Runs in CGB mode only, do speed switch before handing off control.
pub(crate) const CART_CGB_ONLY: u8 = 0xC0;
/// Supports CGB mode but is backwards compatible with monochrome.
pub(crate) const CART_CGB_TOO: u8 = 0x80;

// Interrupt and RST jump targets.
//---------------------------------------------------------
// Interrupt vectors
pub(crate) const INT_VBLANK_VEC: u16 = 0x40;
pub(crate) const INT_STAT_VEC: u16 = 0x48;
pub(crate) const INT_TIMER_VEC: u16 = 0x50;
pub(crate) const INT_SERIAL_VEC: u16 = 0x58;
pub(crate) const INT_JOYPAD_VEC: u16 = 0x60;

// RST jump addresses
// pub(crate) const RST_VECS: [u16; 8] = [
//     0x0000, 0x0008, 0x0010, 0x0018, 0x0020, 0x0028, 0x0030, 0x0038,
// ];
