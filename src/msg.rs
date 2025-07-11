use crate::{info::SCREEN_RESOLUTION, regs};

pub enum Request {
    /// Tell emulator to start executing code
    Start,
    /// Update the emulator state about which buttons are pressed/raised.
    UpdateButtonState(ButtonState),
    /// Cycle through a predefined RGB palette for monochrome ROMs.
    CyclePalette,
    /// Get the latest ready video frame.
    GetVideoFrame,
    /// Get the cartridge title.
    GetTitle,
    /// Get clock frequency
    GetFrequency,
    /// Request a shutdown and wait for [Reply::ShuttingDown] before exiting.
    Shutdown {
        save_state: bool,
    },

    // TODO For debugging the CPU and execution.
    DebuggerStart,
    DebuggerStep,
    DebuggerStop,
}

pub enum Reply {
    /// Video frame in RGB-24.
    VideoFrame(Box<VideoFrame>),
    /// Raw title data stored in the cartridge.
    Title(String),
    /// Current clock frequency.
    Frequency(f64),
    /// Shutdown request acknowledgement message with saved state (if requested).
    ShuttingDown(Option<Box<[u8]>>),
}

#[derive(Clone, bincode::Encode, bincode::Decode)]
pub struct VideoFrame {
    pixels: [[Color; SCREEN_RESOLUTION.0]; SCREEN_RESOLUTION.1],
}

#[derive(Default, Clone, Copy, bincode::Encode, bincode::Decode)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn from_hexcode(hexcode: u32) -> Self {
        let bs = hexcode.to_le_bytes();
        Self {
            r: bs[2],
            g: bs[1],
            b: bs[0],
        }
    }
}

impl VideoFrame {
    pub fn get(&self, x: usize, y: usize) -> Color {
        self.pixels[y][x]
    }

    pub fn set(&mut self, x: usize, y: usize, color: Color) {
        self.pixels[y][x] = color;
    }

    pub fn set_all(&mut self, color: Color) {
        for row in self.pixels.iter_mut() {
            for cell in row.iter_mut() {
                *cell = color;
            }
        }
    }
}

impl Default for VideoFrame {
    fn default() -> Self {
        VideoFrame {
            pixels: [[Default::default(); SCREEN_RESOLUTION.0]; SCREEN_RESOLUTION.1],
        }
    }
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
