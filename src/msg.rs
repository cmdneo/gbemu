use crate::{display, regs};

pub enum UserMsg {
    Buttons(ButtonState),
    ClearFrame(display::Color),
    GetFrame,
    GetFrequency,
    Shutdown,

    // TODO For debugging the CPU and execution.
    DebuggerStart,
    DebuggerStep,
    DebuggerStop,
}

pub enum EmulatorMsg {
    NewFrame(Box<display::Frame>),
    Frequency(f64),
    ShuttingDown,
    Stop,
    WakeUp,
}

/// A glue type for sending button states from user to emulator.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ButtonState {
    // Action buttons
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    // D-Pad buttons
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl ButtonState {
    pub(crate) fn to_internal_repr(self) -> (regs::DPad, regs::ActionButtons) {
        let dpad = regs::DPad {
            right: self.right as u8,
            left: self.left as u8,
            up: self.up as u8,
            down: self.down as u8,
        };

        let btns = regs::ActionButtons {
            a: self.a as u8,
            b: self.b as u8,
            select: self.select as u8,
            start: self.start as u8,
        };

        (dpad, btns)
    }
}
