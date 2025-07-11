mod apu;
mod cartridge;
mod counter;
mod cpu;
mod emulator;
mod info;
mod log;
mod macros;
mod mmu;
mod msg;
mod ppu;
mod regs;
mod serial;
mod timer;

pub use emulator::Emulator;
pub use info::{FREQUENCY, SCREEN_RESOLUTION};
pub use msg::{ButtonState, Color, Reply, Request, VideoFrame};

/// Emulator error type.
#[derive(Debug)]
pub enum EmulatorErr {
    SaveFileCorrupted,
    InvalidRomSize,
    RomSizeMismatch,
    UnknownRomSize,
    UnknownRamSize,
    UnknownMBC,
    NotImplemented,
}

#[inline(always)]
const fn mask_usize(bits: u32) -> usize {
    if bits == usize::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}

#[inline(always)]
const fn mask_u16(bits: u32) -> u16 {
    if bits == u16::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}

#[inline(always)]
const fn mask_u8(bits: u32) -> u8 {
    if bits == u8::BITS {
        !0
    } else {
        !(!0 << bits)
    }
}
