mod apu;
mod cartridge;
mod counter;
mod cpu;
mod info;
mod log;
mod macros;
mod mem;
mod ppu;
mod regs;
mod serial;
mod timer;

// Modules which have public interfaces.
mod emulator;
mod frame;
mod msg;

pub use emulator::Emulator;
pub use frame::{Color, Frame, SCREEN_SIZE};
pub use msg::{ButtonState, EmulatorMsg, UserMsg};

/// Emulator error type.
#[derive(Debug)]
pub enum EmuError {
    UnknownMBC,
}
