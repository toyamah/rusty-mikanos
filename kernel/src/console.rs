use crate::graphics::{PixelColor, PixelWriter};

pub struct Console<'a> {
    writer: &'a PixelWriter<'a>,
    fg_color: PixelColor,
    bg_color: PixelColor,
    cursor_row: usize,
    cursor_column: usize,
    // 1 means null character to be written at end of a line
    buffer: [[char; Console::COLUMNS + 1]; Console::ROWS],
}

impl<'a> Console<'a> {
    const ROWS: usize = 25;
    const COLUMNS: usize = 80;

    pub fn new(
        writer: &'a PixelWriter<'a>,
        fg_color: PixelColor,
        bg_color: PixelColor,
    ) -> Console<'a> {
        Self {
            writer,
            fg_color,
            bg_color,
            cursor_row: 0,
            cursor_column: 0,
            buffer: [[char::from(0); Console::COLUMNS + 1]; Console::ROWS],
        }
    }

    pub fn put_string(&mut self, str: &str) {
        for char in str.chars() {
            if char == '\n' {
                self.new_line();
            } else if self.cursor_column < K_COLUMNS - 1 {
                self.writer.write_ascii(
                    (8 * self.cursor_column) as u32,
                    (16 * self.cursor_row) as u32,
                    char,
                    &self.fg_color,
                );
                self.buffer[self.cursor_row][self.cursor_column] = char;
                self.cursor_column += 1;
            }
        }
    }

    fn new_line(&self) {
        //TODO
    }
}
