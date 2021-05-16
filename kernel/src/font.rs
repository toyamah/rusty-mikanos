use crate::graphics::{PixelColor, PixelWriter};

pub fn write_string(writer: &PixelWriter, x: u32, y: u32, str: &str, color: &PixelColor) {
    for (i, char) in str.chars().enumerate() {
        write_ascii(writer, x + 8 * i as u32, y , char, color);
    }
}

pub fn write_ascii(writer: &PixelWriter, x: u32, y: u32, c: char, color: &PixelColor) {
    let font = unsafe { get_font(c) };
    let font = match font {
        None => return,
        Some(f) => f,
    };

    for dy in 0..16 {
        for dx in 0..8 {
            let bits = unsafe { *font.offset(dy) };
            if (bits << dx) & 0x80 != 0 {
                writer.write(x + dx, y + dy as u32, color);
            }
        }
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
        Some(start.offset(index as isize))
    } else {
        None
    }
}
