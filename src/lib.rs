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
