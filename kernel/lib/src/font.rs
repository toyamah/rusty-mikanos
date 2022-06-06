use crate::graphics::{PixelColor, PixelWriter};

pub fn write_string<W: PixelWriter>(writer: &mut W, x: i32, y: i32, str: &str, color: &PixelColor) {
    let mut offset = 0;
    for (_, char) in str.chars().enumerate() {
        write_ascii(writer, x + 8 * offset as i32, y, char, color);
        offset += if char.is_ascii() { 1 } else { 2 }
    }
}

pub fn write_chars<W: PixelWriter>(
    writer: &mut W,
    x: i32,
    y: i32,
    chars: &[char],
    color: &PixelColor,
) {
    let mut offset = 0;
    for (_, char) in chars.iter().enumerate() {
        write_ascii(writer, x + 8 * offset as i32, y, *char, color);
        offset += if char.is_ascii() { 1 } else { 2 }
    }
}

pub fn write_ascii<W: PixelWriter>(writer: &mut W, x: i32, y: i32, c: char, color: &PixelColor) {
    let font = unsafe { get_font(c) };
    let font = match font {
        None => return,
        Some(f) => f,
    };

    for dy in 0..16 {
        for dx in 0..8 {
            let bits = unsafe { *font.offset(dy) };
            if (bits << dx) & 0x80 != 0 {
                writer.write(x + dx, y + dy as i32, color);
            }
        }
    }
}

pub fn write_unicode<W: PixelWriter>(writer: &mut W, x: i32, y: i32, c: char, color: &PixelColor) {
    if c.is_ascii() {
        write_ascii(writer, x, y, c, color);
    } else {
        write_ascii(writer, x, y, '?', color);
        write_ascii(writer, x + 8, y, '?', color);
    }
}

pub fn count_utf8_size(c: u8) -> usize {
    if c < 0x80 {
        1
    } else if (0xc0..0xe0).contains(&c) {
        2
    } else if (0xe0..0xf0).contains(&c) {
        3
    } else if (0xf0..0xf8).contains(&c) {
        4
    } else {
        0
    }
}

pub fn convert_utf8_to_u32(bytes: &[u8]) -> u32 {
    let at = |i: usize| bytes[i] as u32;

    match count_utf8_size(bytes[0]) {
        1 => at(0),
        2 => (at(0) & 0b0001_1111) << 6 | (at(1) & 0b0011_1111),
        3 => (at(0) & 0b0000_1111) << 12 | (at(1) & 0b0011_1111) << 6 | (at(2) & 0b0011_1111),
        4 => {
            (at(0) & 0b0000_0111) << 18
                | (at(1) & 0b0011_1111) << 12
                | (at(2) & 0b0011_1111) << 6
                | (at(3) & 0b0011_1111)
        }
        _ => 0,
    }
}

extern "C" {
    static _binary_hankaku_bin_start: u8;
    static _binary_hankaku_bin_end: u8;
    static _binary_hankaku_bin_size: u8;
}

unsafe fn get_font(c: char) -> Option<*mut u8> {
    let index = 16 * c as usize;
    let size = (&_binary_hankaku_bin_size as *const u8) as usize;

    if index < size {
        let start = (&_binary_hankaku_bin_start as *const u8) as *mut u8;
        Some(start.add(index))
    } else {
        None
    }
}
