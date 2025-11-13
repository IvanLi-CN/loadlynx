#![allow(dead_code)]

mod data {
    include!("fonts_data.rs");
}

pub struct UtftFont {
    width: u8,
    height: u8,
    first_char: u8,
    char_count: u8,
    raw: &'static [u8],
}

impl UtftFont {
    pub const fn from_bytes(bytes: &'static [u8]) -> Self {
        let width = bytes[0];
        let height = bytes[1];
        let first_char = bytes[2];
        let char_count = bytes[3];
        Self {
            width,
            height,
            first_char,
            char_count,
            raw: bytes,
        }
    }

    pub fn width(&self) -> u8 {
        self.width
    }

    pub fn height(&self) -> u8 {
        self.height
    }

    pub fn draw_char<F: FnMut(i32, i32)>(
        &self,
        ch: char,
        mut plot: F,
        origin_x: i32,
        origin_y: i32,
    ) {
        let code = ch as u32;
        if code < self.first_char as u32 {
            return;
        }
        let idx = code - self.first_char as u32;
        if idx >= self.char_count as u32 {
            return;
        }

        let bytes_per_row = ((self.width as usize) + 7) / 8;
        let glyph_size = bytes_per_row * self.height as usize;
        let glyph_offset = 4 + idx as usize * glyph_size;
        let glyph = &self.raw[glyph_offset..glyph_offset + glyph_size];

        for row in 0..self.height as usize {
            for col in 0..self.width as usize {
                let byte = glyph[row * bytes_per_row + (col / 8)];
                let bit = 7 - (col % 8);
                if (byte >> bit) & 0x01 != 0 {
                    let x = origin_x + col as i32;
                    let y = origin_y + row as i32;
                    plot(x, y);
                }
            }
        }
    }
}

pub const SMALL_FONT: UtftFont = UtftFont::from_bytes(&data::SMALL_FONT);
pub const SEVEN_SEG_FONT: UtftFont = UtftFont::from_bytes(&data::SEVEN_SEG);
