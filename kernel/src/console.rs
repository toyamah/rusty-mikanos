use crate::{console, layer_manager_op};
use alloc::format;
use core::fmt;
use core::fmt::Write;
use lib::graphics::{
    fill_rectangle, FrameBufferWriter, PixelColor, PixelWriter, Rectangle, Vector2D,
    DESKTOP_BG_COLOR,
};
use lib::timer::measure_time;
use lib::window::Window;

const ROWS: usize = 25;
const COLUMNS: usize = 80;

pub struct Console<'a> {
    writer: ConsoleWriter<'a>,
    fg_color: PixelColor,
    bg_color: PixelColor,
    cursor_row: usize,
    cursor_column: usize,
    // 1 means null character to be written at end of a line
    buffer: [[char; COLUMNS + 1]; ROWS],
}

impl<'a> Console<'a> {
    pub fn new(
        writer: &'a FrameBufferWriter,
        fg_color: PixelColor,
        bg_color: PixelColor,
    ) -> Console<'a> {
        Self {
            writer: ConsoleWriter::FrameBufferWriter(writer),
            fg_color,
            bg_color,
            cursor_row: 0,
            cursor_column: 0,
            buffer: [[char::from(0); COLUMNS + 1]; ROWS],
        }
    }

    pub fn reset_window(&mut self, window: &'a mut Window) {
        self.writer = ConsoleWriter::Window(window);
        self.refresh();
    }

    pub fn put_string(&mut self, str: &str) {
        for char in str.chars() {
            if char == '\n' {
                self.new_line();
            } else if self.cursor_column < COLUMNS - 1 {
                self.writer.write_ascii(
                    (8 * self.cursor_column) as i32,
                    (16 * self.cursor_row) as i32,
                    char,
                    &self.fg_color,
                );
                self.buffer[self.cursor_row][self.cursor_column] = char;
                self.cursor_column += 1;
            }
        }

        if let Some(m) = layer_manager_op() {
            m.draw();
        }
    }

    fn new_line(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < ROWS - 1 {
            self.cursor_row += 1;
            return;
        }

        match &self.writer {
            ConsoleWriter::Window(w) => {
                let rows = ROWS as i32;
                let columns = COLUMNS as i32;
                let move_src = Rectangle::new(
                    Vector2D::new(0, 16),
                    Vector2D::new(8 * columns, 16 * (rows - 1)),
                );
                w.move_(Vector2D::new(0, 0), &move_src);
                fill_rectangle(
                    &self.writer,
                    &Vector2D::new(0, 16 * (rows - 1)),
                    &Vector2D::new(8 * columns, 16),
                    &DESKTOP_BG_COLOR,
                );
            }
            ConsoleWriter::FrameBufferWriter(_) => {
                fill_rectangle(
                    &self.writer,
                    &Vector2D::new(0, 0),
                    &Vector2D::new((8 * COLUMNS) as i32, (16 * ROWS) as i32),
                    &DESKTOP_BG_COLOR,
                );
                for row in 0..ROWS - 1 {
                    let next = row + 1;
                    self.buffer.copy_within(next..=next, row);
                    self.writer.write_chars(
                        0,
                        (16 * row) as i32,
                        &self.buffer[row],
                        &self.fg_color,
                    );
                }
                self.buffer[ROWS - 1].fill(char::from(0));
            }
        }
    }

    fn refresh(&mut self) {
        for (i, row) in self.buffer.iter().enumerate() {
            self.writer
                .write_chars(0, (16 * i) as i32, row, &self.fg_color);
        }
    }
}

enum ConsoleWriter<'a> {
    FrameBufferWriter(&'a FrameBufferWriter),
    Window(&'a mut Window),
}

impl<'a> PixelWriter for ConsoleWriter<'a> {
    fn write(&self, x: i32, y: i32, color: &PixelColor) {
        match self {
            ConsoleWriter::FrameBufferWriter(w) => w.write(x, y, color),
            ConsoleWriter::Window(w) => w.write(x, y, color),
        }
    }

    fn width(&self) -> i32 {
        match self {
            ConsoleWriter::FrameBufferWriter(w) => w.width(),
            ConsoleWriter::Window(w) => w.width(),
        }
    }

    fn height(&self) -> i32 {
        match self {
            ConsoleWriter::FrameBufferWriter(w) => w.height(),
            ConsoleWriter::Window(w) => w.height(),
        }
    }
}

impl<'a> fmt::Write for Console<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(s);
        Ok(())
    }
}

pub fn _printk(args: fmt::Arguments) {
    // let time = measure_time(|| console().write_fmt(args).unwrap());
    // console().write_fmt(format_args!("[{:#09}]", time)).unwrap();

    // To draw text rapidly, avoid using write_fmt
    // because write_fmt calls write_str for every argument and then LayoutManager.draw() is called as many times as the argument's size.
    let text = format!("{}", args);
    let time = measure_time(|| console().write_str(&text).unwrap());
    let text = format!("{}", format_args!("[{:#09}]", time));
    console().write_str(&text).unwrap();
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ($crate::console::_printk(format_args!($($arg)*)));
}
