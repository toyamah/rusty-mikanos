use crate::font;
use shared::{FrameBufferConfig, PixelFormat};

pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

impl PixelColor {
    pub fn new(r: u8, g: u8, b: u8) -> PixelColor {
        PixelColor { r, g, b }
    }
}

pub struct PixelWriter<'a> {
    config: &'a FrameBufferConfig,
    write_fn: fn(&Self, x: u32, y: u32, &PixelColor) -> (),
}

impl<'a> PixelWriter<'a> {
    pub fn new(config: &'a FrameBufferConfig) -> Self {
        Self {
            config,
            write_fn: match config.pixel_format {
                PixelFormat::KPixelRGBResv8BitPerColor => Self::write_rgb,
                PixelFormat::KPixelBGRResv8BitPerColor => Self::write_bgr,
            },
        }
    }

    pub fn write_string(&self, x: u32, y: u32, str: &str, color: &PixelColor) {
        font::write_string(self, x, y, str, color);
    }

    pub fn write_chars(&self, x: u32, y: u32, chars: &[char], color: &PixelColor) {
        font::write_chars(self, x, y, chars, color);
    }

    pub fn write_ascii(&self, x: u32, y: u32, char: char, color: &PixelColor) {
        font::write_ascii(self, x, y, char, color);
    }

    pub fn write(&self, x: u32, y: u32, color: &PixelColor) {
        (self.write_fn)(self, x, y, color);
    }

    fn write_rgb(self: &Self, x: u32, y: u32, color: &PixelColor) {
        let p = self.pixel_at(x, y);
        unsafe {
            *p.offset(0) = color.r;
            *p.offset(1) = color.g;
            *p.offset(2) = color.b;
        }
    }

    fn write_bgr(self: &Self, x: u32, y: u32, color: &PixelColor) {
        let p = self.pixel_at(x, y);
        unsafe {
            *p.offset(0) = color.b;
            *p.offset(1) = color.g;
            *p.offset(2) = color.r;
        }
    }

    fn pixel_at(&self, x: u32, y: u32) -> *mut u8 {
        let pixel_position = self.config.pixels_per_scan_line * y + x;
        let base = (4 * pixel_position) as isize;
        unsafe { self.config.frame_buffer.offset(base) }
    }
}
