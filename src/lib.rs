pub(crate) mod cartridge;
pub(crate) mod cpu;
pub(crate) mod info;
pub(crate) mod log;
pub(crate) mod macros;
pub(crate) mod mem;
pub(crate) mod ppu;
pub(crate) mod regs;
pub(crate) mod timer;

pub mod display;
pub mod emulator;
pub mod msg;

/// Emulator error type.
#[derive(Debug)]
pub enum EmuError {
    UnknownMBC,
}
