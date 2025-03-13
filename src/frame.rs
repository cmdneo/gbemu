use crate::info::SCREEN_RESOLUTION;

pub const SCREEN_SIZE: (usize, usize) = SCREEN_RESOLUTION;

#[derive(Clone)]
pub struct Frame {
    pixels: [[Color; SCREEN_RESOLUTION.0]; SCREEN_RESOLUTION.1],
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
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

    #[inline]
    pub fn to_f32_triple(self) -> (f32, f32, f32) {
        (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        )
    }
}

impl Frame {
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

impl Default for Frame {
    fn default() -> Self {
        Frame {
            pixels: [[Default::default(); SCREEN_RESOLUTION.0]; SCREEN_RESOLUTION.1],
        }
    }
}
