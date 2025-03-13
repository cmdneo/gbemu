use crate::{frame, regs};

pub enum UserMsg {
    UpdateButtons(ButtonState),
    CyclePalette,
    ClearFrame(frame::Color),
    GetFrame,
    GetFrequency,
    Shutdown,

    // TODO For debugging the CPU and execution.
    DebuggerStart,
    DebuggerStep,
    DebuggerStop,
}

pub enum EmulatorMsg {
    NewFrame(Box<frame::Frame>),
    Frequency(f64),
    ShuttingDown,
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

// /// Some emulator state information.
// #[derive(Default, Clone, Copy)]
// pub struct EmulatorInfo {
//     channel_averages: [f32; 4],
// }
