use crate::font;
use core::cmp::{max, min};
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

impl From<u32> for PixelColor {
    fn from(v: u32) -> Self {
        PixelColor::new(
            (v >> 16 & 0xff) as u8,
            (v >> 8 & 0xff) as u8,
            (v & 0xff) as u8,
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vector2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vector2D<T> {
    pub const fn new(x: T, y: T) -> Vector2D<T> {
        Self { x, y }
    }
}

impl Vector2D<usize> {
    pub fn to_i32_vec2d(&self) -> Vector2D<i32> {
        Vector2D::new(self.x as i32, self.y as i32)
    }
}

impl<T> Vector2D<T>
where
    T: Copy + Ord,
{
    #[must_use]
    pub fn element_max(&self, other: Vector2D<T>) -> Vector2D<T> {
        Vector2D::new(max(self.x, other.x), max(self.y, other.y))
    }

    #[must_use]
    pub fn element_min(&self, other: Vector2D<T>) -> Vector2D<T> {
        Vector2D::new(min(self.x, other.x), min(self.y, other.y))
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rectangle<T> {
    pub pos: Vector2D<T>,
    pub size: Vector2D<T>,
}

impl<T> Rectangle<T> {
    pub fn new(pos: Vector2D<T>, size: Vector2D<T>) -> Rectangle<T> {
        Self { pos, size }
    }
}

impl PixelColor {
    pub const fn new(r: u8, g: u8, b: u8) -> PixelColor {
        PixelColor { r, g, b }
    }
}

pub trait PixelWriter {
    fn write(&mut self, x: i32, y: i32, color: &PixelColor);
    fn width(&self) -> i32;
    fn height(&self) -> i32;

    fn write_string(&mut self, x: i32, y: i32, str: &str, color: &PixelColor) {
        font::write_string(self, x, y, str, color);
    }

    fn write_chars(&mut self, x: i32, y: i32, chars: &[char], color: &PixelColor) {
        font::write_chars(self, x, y, chars, color);
    }

    fn write_ascii(&mut self, x: i32, y: i32, c: char, color: &PixelColor) {
        font::write_ascii(self, x, y, c, color);
    }
}

pub struct FrameBufferWriter {
    config: FrameBufferConfig,
    write_fn: fn(&Self, x: i32, y: i32, &PixelColor) -> (),
}

impl PixelWriter for FrameBufferWriter {
    fn write(&mut self, x: i32, y: i32, color: &PixelColor) {
        (self.write_fn)(self, x, y, color);
    }

    fn width(&self) -> i32 {
        self.config.horizontal_resolution as i32
    }

    fn height(&self) -> i32 {
        self.config.vertical_resolution as i32
    }
}

impl FrameBufferWriter {
    pub fn new(config: FrameBufferConfig) -> Self {
        Self {
            config,
            write_fn: match config.pixel_format {
                PixelFormat::KPixelRGBResv8BitPerColor => Self::write_rgb,
                PixelFormat::KPixelBGRResv8BitPerColor => Self::write_bgr,
            },
        }
    }

    fn write_rgb(&self, x: i32, y: i32, color: &PixelColor) {
        let p = self.pixel_at(x, y);
        unsafe {
            *p.offset(0) = color.r;
            *p.offset(1) = color.g;
            *p.offset(2) = color.b;
        }
    }

    fn write_bgr(&self, x: i32, y: i32, color: &PixelColor) {
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

pub fn draw_desktop<W: PixelWriter>(writer: &mut W) {
    let width = writer.width();
    let height = writer.height();
    fill_rectangle(
        writer,
        &Vector2D::new(0, 0),
        &Vector2D::new(width, height),
        &DESKTOP_BG_COLOR,
    );
    fill_rectangle(
        writer,
        &Vector2D::new(0, height - 50),
        &Vector2D::new(width, 50),
        &PixelColor::new(1, 8, 17),
    );
    fill_rectangle(
        writer,
        &Vector2D::new(0, height - 50),
        &Vector2D::new(width / 5, 50),
        &PixelColor::new(80, 80, 80),
    );
    draw_rectangle(
        writer,
        &Vector2D::new(10, height - 40),
        &Vector2D::new(30, 30),
        &PixelColor::new(160, 160, 160),
    );
}

pub fn fill_rectangle<W: PixelWriter>(
    writer: &mut W,
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

fn draw_rectangle<W: PixelWriter>(
    writer: &mut W,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_color_from() {
        let p = PixelColor::from(0x123456);
        assert_eq!(p, PixelColor::new(0x12, 0x34, 0x56));
    }

    #[test]
    fn vector2d_element_max() {
        let max = Vector2D::new(100, 20).element_max(Vector2D::new(10, 200));
        assert_eq!(Vector2D::new(100, 200), max);

        let max = Vector2D::new(11, 222).element_max(Vector2D::new(111, 22));
        assert_eq!(Vector2D::new(111, 222), max);

        let max = Vector2D::new(1, 2).element_max(Vector2D::new(1, 2));
        assert_eq!(Vector2D::new(1, 2), max);
    }

    #[test]
    fn vector2d_element_min() {
        let min = Vector2D::new(100, 20).element_min(Vector2D::new(10, 200));
        assert_eq!(Vector2D::new(10, 20), min);

        let min = Vector2D::new(11, 222).element_min(Vector2D::new(111, 22));
        assert_eq!(Vector2D::new(11, 22), min);

        let min = Vector2D::new(1, 2).element_min(Vector2D::new(1, 2));
        assert_eq!(Vector2D::new(1, 2), min);
    }
}
