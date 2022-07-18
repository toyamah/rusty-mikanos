use crate::console::Mode::{ConsoleWindow, Frame};
use crate::font::{write_ascii, write_chars};
use crate::graphics::global::pixel_writer;
use crate::graphics::{fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D};
use crate::layer::global::layer_manager_op;
use crate::layer::LayerID;
use crate::message::{LayerMessage, LayerOperation, Message, MessageType};
use crate::sync::Mutex;
use crate::task::global::{main_task_id, task_manager};
use crate::Window;
use alloc::sync::Arc;
use core::arch::asm;
use core::fmt;
use shared::PixelFormat;

const ROWS: usize = 25;
const COLUMNS: usize = 80;

pub mod global {
    use super::Console;
    use crate::graphics::{DESKTOP_BG_COLOR, DESKTOP_FG_COLOR};
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
        console().write_fmt(args).unwrap();
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

pub enum Mode {
    Frame,
    ConsoleWindow(Arc<Mutex<Window>>),
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

    pub fn reset_mode(&mut self, mode: Mode) {
        self.mode = mode;
        match &self.mode {
            Frame => {}
            ConsoleWindow(w) => {
                let w = Arc::clone(w);
                self.refresh(w.lock().writer());
            }
        }
    }

    pub fn layer_id(&self) -> Option<LayerID> {
        self.layer_id
    }

    pub fn set_layer_id(&mut self, layer_id: LayerID) {
        self.layer_id = Some(layer_id);
    }

    fn put_string(&mut self, str: &str) {
        for char in str.chars() {
            if char == '\n' {
                self.new_line();
            } else if self.cursor_column < COLUMNS - 1 {
                match &self.mode {
                    Frame => write_ascii(
                        pixel_writer(),
                        (8 * self.cursor_column) as i32,
                        (16 * self.cursor_row) as i32,
                        char,
                        &self.fg_color,
                    ),
                    ConsoleWindow(w) => write_ascii(
                        w.lock().writer(),
                        (8 * self.cursor_column) as i32,
                        (16 * self.cursor_row) as i32,
                        char,
                        &self.fg_color,
                    ),
                };
                self.buffer[self.cursor_row][self.cursor_column] = char;
                self.cursor_column += 1;
            }
        }

        if let Some(m) = layer_manager_op() {
            if let Some(id) = self.layer_id {
                m.lock().draw_layer_of(id);
            }
        }
    }

    fn new_line(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < ROWS - 1 {
            self.cursor_row += 1;
            return;
        }

        match &self.mode {
            ConsoleWindow(window) => {
                let rows = ROWS as i32;
                let columns = COLUMNS as i32;
                let move_src = Rectangle::new(
                    Vector2D::new(0, 16),
                    Vector2D::new(8 * columns, 16 * (rows - 1)),
                );
                let mut w = window.lock();
                w.move_(Vector2D::new(0, 0), &move_src);
                fill_rectangle(
                    w.writer(),
                    &Vector2D::new(0, 16 * (rows - 1)),
                    &Vector2D::new(8 * columns, 16),
                    &self.bg_color,
                );
            }
            Frame => {
                fill_rectangle(
                    pixel_writer(),
                    &Vector2D::new(0, 0),
                    &Vector2D::new((8 * COLUMNS) as i32, (16 * ROWS) as i32),
                    &self.bg_color,
                );
                for row in 0..ROWS - 1 {
                    let next = row + 1;
                    self.buffer.copy_within(next..=next, row);
                    write_chars(
                        pixel_writer(),
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

    fn refresh<W: PixelWriter>(&mut self, writer: &mut W) {
        fill_rectangle(
            writer,
            &Vector2D::new(0, 0),
            &Vector2D::new((8 * COLUMNS) as i32, (16 * ROWS) as i32),
            &self.bg_color,
        );
        for (i, row) in self.buffer.iter().enumerate() {
            write_chars(writer, 0, (16 * i) as i32, row, &self.fg_color);
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.put_string(s);
        Ok(())
    }
}

pub fn new_console_window(format: PixelFormat) -> Window {
    Window::new(COLUMNS * 8, ROWS * 16, format)
}
