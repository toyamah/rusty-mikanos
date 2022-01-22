use crate::font;
use core::ops::{Add, AddAssign};
use shared::{FrameBufferConfig, PixelFormat};

pub const COLOR_BLACK: PixelColor = PixelColor {
    r: 255,
    g: 255,
    b: 255,
};
pub const COLOR_WHITE: PixelColor = PixelColor { r: 0, g: 0, b: 0 };

pub const DESKTOP_BG_COLOR: PixelColor = PixelColor {
    r: 45,
    g: 118,
    b: 237,
};
pub const DESKTOP_FG_COLOR: PixelColor = COLOR_BLACK;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct Vector2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2D<T> {
    pub fn new(x: T, y: T) -> Vector2D<T> {
        Self { x, y }
    }
}

impl<T> Add for Vector2D<T>
where
    T: Add<Output = T> + Copy + Clone,
{
    type Output = Vector2D<T>;

    fn add(self, other: Self) -> Self::Output {
        Vector2D::<T> {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl<T> AddAssign for Vector2D<T>
where
    T: AddAssign + Copy + Clone,
{
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl PixelColor {
    pub const fn new(r: u8, g: u8, b: u8) -> PixelColor {
        PixelColor { r, g, b }
    }
}

pub trait PixelWriter {
    fn write(&self, x: i32, y: i32, color: &PixelColor);
    fn width(&self) -> i32;
    fn height(&self) -> i32;
}

pub struct FrameBufferWriter<'a> {
    config: &'a FrameBufferConfig,
    write_fn: fn(&Self, x: i32, y: i32, &PixelColor) -> (),
}

impl<'a> PixelWriter for FrameBufferWriter<'a> {
    fn write(&self, x: i32, y: i32, color: &PixelColor) {
        (self.write_fn)(self, x, y, color);
    }

    fn width(&self) -> i32 {
        self.config.horizontal_resolution as i32
    }

    fn height(&self) -> i32 {
        self.config.vertical_resolution as i32
    }
}

impl<'a> FrameBufferWriter<'a> {
    pub fn new(config: &'a FrameBufferConfig) -> Self {
        Self {
            config,
            write_fn: match config.pixel_format {
                PixelFormat::KPixelRGBResv8BitPerColor => Self::write_rgb,
                PixelFormat::KPixelBGRResv8BitPerColor => Self::write_bgr,
            },
        }
    }

    pub fn write_string(&self, x: i32, y: i32, str: &str, color: &PixelColor) {
        font::write_string(self, x, y, str, color);
    }

    pub fn write_chars(&self, x: i32, y: i32, chars: &[char], color: &PixelColor) {
        font::write_chars(self, x, y, chars, color);
    }

    pub fn write_ascii(&self, x: i32, y: i32, char: char, color: &PixelColor) {
        font::write_ascii(self, x, y, char, color);
    }

    fn write_rgb(self: &Self, x: i32, y: i32, color: &PixelColor) {
        let p = self.pixel_at(x, y);
        unsafe {
            *p.offset(0) = color.r;
            *p.offset(1) = color.g;
            *p.offset(2) = color.b;
        }
    }

    fn write_bgr(self: &Self, x: i32, y: i32, color: &PixelColor) {
        let p = self.pixel_at(x, y);
        unsafe {
            *p.offset(0) = color.b;
            *p.offset(1) = color.g;
            *p.offset(2) = color.r;
        }
    }

    fn pixel_at(&self, x: i32, y: i32) -> *mut u8 {
        let pixel_position = self.config.pixels_per_scan_line as i32 * y + x;
        let base = (4 * pixel_position) as isize;
        unsafe { self.config.frame_buffer.offset(base) }
    }
}

pub fn fill_rectangle<W: PixelWriter>(
    writer: &W,
    pos: &Vector2D<i32>,
    size: &Vector2D<i32>,
    c: &PixelColor,
) {
    for dy in 0..size.y {
        for dx in 0..size.x {
            writer.write(pos.x + dx, pos.y + dy, c);
        }
    }
}

pub fn draw_rectangle<W: PixelWriter>(
    writer: &W,
    pos: &Vector2D<i32>,
    size: &Vector2D<i32>,
    c: &PixelColor,
) {
    for dx in 0..size.x {
        writer.write(pos.x + dx, pos.y, c);
        writer.write(pos.x + dx, pos.y + size.y - 1, c);
    }
    for dy in 0..size.y {
        writer.write(pos.x, pos.y + dy, c);
        writer.write(pos.x + size.x - 1, pos.y + dy, c);
    }
}
