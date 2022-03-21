use crate::console::Mode::{ConsoleWindow, Frame};
use crate::graphics::global::pixel_writer;
use crate::graphics::{fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D};
use crate::layer::global::{console_window, layer_manager_op, screen_frame_buffer};
use crate::layer::LayerID;
use crate::Window;
use core::fmt;
use shared::PixelFormat;

const ROWS: usize = 25;
const COLUMNS: usize = 80;

pub mod global {
    use super::Console;
    use crate::graphics::{DESKTOP_BG_COLOR, DESKTOP_FG_COLOR};
    use alloc::format;
    use core::fmt;
    use core::fmt::Write;

    static mut CONSOLE: Option<Console> = None;
    pub fn console() -> &'static mut Console {
        unsafe { CONSOLE.as_mut().unwrap() }
    }

    pub fn initialize() {
        unsafe { CONSOLE = Some(Console::new(DESKTOP_FG_COLOR, DESKTOP_BG_COLOR)) }
    }

    pub fn _printk(args: fmt::Arguments) {
        // let time = measure_time(|| console().write_fmt(args).unwrap());
        // console().write_fmt(format_args!("[{:#09}]", time)).unwrap();

        // To draw text rapidly, avoid using write_fmt
        // because write_fmt calls write_str for every argument and then LayoutManager.draw() is called as many times as the argument's size.
        let text = format!("{}", args);
        console().write_str(&text).unwrap();
    }
}

pub struct Console {
    mode: Mode,
    fg_color: PixelColor,
    bg_color: PixelColor,
    cursor_row: usize,
    cursor_column: usize,
    layer_id: Option<LayerID>,
    // 1 means null character to be written at end of a line
    buffer: [[char; COLUMNS + 1]; ROWS],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    Frame,
    ConsoleWindow,
}

impl Console {
    pub fn new(fg_color: PixelColor, bg_color: PixelColor) -> Console {
        Self {
            mode: Frame,
            fg_color,
            bg_color,
            cursor_row: 0,
            cursor_column: 0,
            layer_id: None,
            buffer: [[char::from(0); COLUMNS + 1]; ROWS],
        }
    }

    pub fn reset_mode<W: PixelWriter>(&mut self, mode: Mode, writer: &mut W) {
        self.mode = mode;
        self.refresh(writer);
    }

    pub fn layer_id(&self) -> Option<LayerID> {
        self.layer_id
    }

    pub fn set_layer_id(&mut self, layer_id: LayerID) {
        self.layer_id = Some(layer_id);
    }

    fn put_string<W: PixelWriter>(&mut self, str: &str, writer: &mut W) {
        for char in str.chars() {
            if char == '\n' {
                self.new_line(writer);
            } else if self.cursor_column < COLUMNS - 1 {
                writer.write_ascii(
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
            if let Some(id) = self.layer_id {
                m.draw_layer_of(id, screen_frame_buffer());
            }
        }
    }

    fn new_line<W: PixelWriter>(&mut self, writer: &mut W) {
        self.cursor_column = 0;
        if self.cursor_row < ROWS - 1 {
            self.cursor_row += 1;
            return;
        }

        match self.mode {
            ConsoleWindow => {
                let rows = ROWS as i32;
                let columns = COLUMNS as i32;
                let move_src = Rectangle::new(
                    Vector2D::new(0, 16),
                    Vector2D::new(8 * columns, 16 * (rows - 1)),
                );
                // TODO: take off referencing a global var if possible
                console_window().move_(Vector2D::new(0, 0), &move_src);
                fill_rectangle(
                    writer,
                    &Vector2D::new(0, 16 * (rows - 1)),
                    &Vector2D::new(8 * columns, 16),
                    &self.bg_color,
                );
            }
            Frame => {
                fill_rectangle(
                    writer,
                    &Vector2D::new(0, 0),
                    &Vector2D::new((8 * COLUMNS) as i32, (16 * ROWS) as i32),
                    &self.bg_color,
                );
                for row in 0..ROWS - 1 {
                    let next = row + 1;
                    self.buffer.copy_within(next..=next, row);
                    writer.write_chars(0, (16 * row) as i32, &self.buffer[row], &self.fg_color);
                }
                self.buffer[ROWS - 1].fill(char::from(0));
            }
        }
    }

    fn refresh<W: PixelWriter>(&mut self, writer: &mut W) {
        fill_rectangle(
            writer,
            &Vector2D::new(0, 0),
            &Vector2D::new((8 * COLUMNS) as i32, (16 * ROWS) as i32),
            &self.bg_color,
        );
        for (i, row) in self.buffer.iter().enumerate() {
            writer.write_chars(0, (16 * i) as i32, row, &self.fg_color);
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.mode {
            Frame => self.put_string(s, pixel_writer()),
            ConsoleWindow => self.put_string(s, console_window()),
        }
        Ok(())
    }
}

pub fn new_console_window(format: PixelFormat) -> Window {
    Window::new(COLUMNS * 8, ROWS * 16, format)
}
