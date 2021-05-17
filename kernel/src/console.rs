use crate::graphics::{PixelColor, PixelWriter};

const ROWS: usize = 25;
const COLUMNS: usize = 80;

pub struct Console<'a> {
    writer: &'a PixelWriter<'a>,
    fg_color: PixelColor,
    bg_color: PixelColor,
    cursor_row: usize,
    cursor_column: usize,
    // 1 means null character to be written at end of a line
    buffer: [[char; COLUMNS + 1]; ROWS],
}

impl<'a> Console<'a> {
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
            buffer: [[char::from(0); COLUMNS + 1]; ROWS],
        }
    }

    pub fn put_string(&mut self, str: &str) {
        for char in str.chars() {
            if char == '\n' {
                self.new_line();
            } else if self.cursor_column < COLUMNS - 1 {
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

    fn new_line(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < ROWS - 1 {
            self.cursor_row += 1;
            return;
        }

        for y in 0..16 * ROWS {
            for x in 0..8 * COLUMNS {
                self.writer.write(x as u32, y as u32, &self.bg_color);
            }
        }

        for row in 0..ROWS - 1 {
            let next = row + 1;
            self.buffer.copy_within(next..=next, row);
            self.writer
                .write_chars(0, (16 * row) as u32, &self.buffer[row], &self.fg_color);
        }

        self.buffer[ROWS - 1].fill(char::from(0));
    }
}