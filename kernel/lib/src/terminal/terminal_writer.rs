use crate::font::{write_ascii, write_string, write_unicode};
use crate::graphics::{fill_rectangle, Rectangle, Vector2D, COLOR_BLACK, COLOR_WHITE};
use crate::layer::global::layer_manager;
use crate::layer::LayerID;
use crate::message::{LayerMessage, LayerOperation, Message, MessageType};
use crate::task::global::task_manager;
use crate::task::TaskID;
use crate::terminal::lib::{COLUMNS, ROWS};
use crate::window::{TITLED_WINDOW_BOTTOM_RIGHT_MARGIN, TITLED_WINDOW_TOP_LEFT_MARGIN};
use crate::Window;
use alloc::collections::BTreeMap;
use core::arch::asm;
use core::fmt::Write;
use spin::Mutex;

// pub(super) static TERMINAL_WRITERS: RwLock<TerminalWriters> = RwLock::new(TerminalWriters::new());
pub(super) static mut TERMINAL_WRITERS: TerminalWriters = TerminalWriters::new();

pub(super) struct TerminalWriters(pub BTreeMap<TaskID, Mutex<TerminalWriter>>);

impl TerminalWriters {
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn register(&mut self, task_id: TaskID, writer: TerminalWriter) {
        unsafe { asm!("cli") };
        let registered = self.0.insert(task_id, Mutex::new(writer)).is_some();
        if registered {
            panic!("TerminalWriter of {:?} has already registered", task_id);
        }
        unsafe { asm!("sti") };
    }

    pub fn get(&self, task_id: TaskID) -> &Mutex<TerminalWriter> {
        self.0.get(&task_id).unwrap()
    }

    pub fn remove(&mut self, task_id: TaskID) {
        unsafe { asm!("cli") };
        let _ = self.0.remove(&task_id);
        unsafe { asm!("sti") };
    }
}

pub(super) struct TerminalWriter {
    layer_id: LayerID,
    task_id: TaskID,
    cursor: Vector2D<i32>,
    is_cursor_visible: bool,
}

impl TerminalWriter {
    pub fn new(layer_id: LayerID, task_id: TaskID) -> Self {
        Self {
            layer_id,
            task_id,
            cursor: Vector2D::new(0, 0),
            is_cursor_visible: false,
        }
    }

    pub fn print(&mut self, s: &str) {
        let prev_cursor = self.calc_cursor_pos();
        self.draw_cursor(false);

        for char in s.chars() {
            self.print_char(char);
        }

        self.draw_cursor(true);
        let current_cursor = self.calc_cursor_pos();

        let draw_pos = Vector2D::new(TITLED_WINDOW_TOP_LEFT_MARGIN.x, prev_cursor.y);
        let draw_size = Vector2D::new(
            self.window_mut()
                .map(|w| w.inner_size().x)
                .unwrap_or(-TITLED_WINDOW_TOP_LEFT_MARGIN.x - TITLED_WINDOW_BOTTOM_RIGHT_MARGIN.x),
            current_cursor.y - prev_cursor.y + 16,
        );
        let msg = Message::new(MessageType::Layer(LayerMessage {
            layer_id: self.layer_id,
            op: LayerOperation::DrawArea(Rectangle::new(draw_pos, draw_size)),
            src_task_id: self.task_id,
        }));

        unsafe { asm!("cli") };
        task_manager()
            .send_message(task_manager().main_task().id(), msg)
            .unwrap();
        unsafe { asm!("sti") };
    }

    pub fn print_char(&mut self, c: char) {
        let window = match self.window_mut() {
            None => return,
            Some(w) => w,
        };

        if c == '\n' {
            self.new_line();
            return;
        }

        let columns = COLUMNS as i32;
        if c.is_ascii() {
            if self.cursor.x == columns {
                self.new_line();
            }
            let pos = self.calc_cursor_pos();
            write_unicode(
                &mut window.normal_window_writer(),
                pos.x,
                pos.y,
                c,
                &COLOR_WHITE,
            )
            .unwrap_or_default();
            self.cursor.x += 1;
        } else {
            if self.cursor.x == columns - 1 {
                self.new_line();
            }
            let pos = self.calc_cursor_pos();
            write_unicode(
                &mut window.normal_window_writer(),
                pos.x,
                pos.y,
                c,
                &COLOR_WHITE,
            )
            .unwrap_or_default();
            self.cursor.x += 2;
        }
    }

    pub fn blink_cursor(&mut self) -> Rectangle<i32> {
        self.is_cursor_visible = !self.is_cursor_visible;
        self.draw_cursor(self.is_cursor_visible);
        Rectangle::new(self.calc_cursor_pos(), Vector2D::new(7, 15))
    }

    pub fn calc_cursor_pos(&self) -> Vector2D<i32> {
        TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4 + 8 * self.cursor.x, 4 + 16 * self.cursor.y)
    }

    pub fn scroll1(&mut self) {
        if let Some(window) = self.window_mut() {
            let move_src = Rectangle::new(
                TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4, 4 + 16),
                Vector2D::new(8 * COLUMNS as i32, 16 * (ROWS as i32 - 1)),
            );
            window.move_(
                TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4, 4),
                &move_src,
            );
            fill_rectangle(
                window,
                &Vector2D::new(4, 4 + 16 * self.cursor.y),
                &Vector2D::new(8 * COLUMNS as i32, 16),
                &COLOR_BLACK,
            );
        }
    }

    pub fn draw_cursor(&mut self, visible: bool) {
        if let Some(window) = self.window_mut() {
            let color = if visible { &COLOR_WHITE } else { &COLOR_BLACK };
            fill_rectangle(
                &mut window.normal_window_writer(),
                &self.calc_cursor_pos(),
                &Vector2D::new(7, 15),
                color,
            );
        }
    }

    pub fn new_line(&mut self) {
        self.cursor.x = 0;
        if self.cursor.y < ROWS as i32 - 1 {
            self.cursor.y += 1;
        } else {
            self.scroll1()
        }
    }

    pub(super) fn redraw(&mut self) {
        let size = match self.window_mut() {
            None => return,
            Some(w) => w.inner_size(),
        };
        let draw_area = Rectangle::new(TITLED_WINDOW_TOP_LEFT_MARGIN, size);

        let msg = Message::new(MessageType::Layer(LayerMessage {
            layer_id: self.layer_id,
            op: LayerOperation::DrawArea(draw_area),
            src_task_id: self.task_id,
        }));

        unsafe { asm!("cli") };
        let _ = task_manager().send_message(task_manager().main_task().id(), msg);
        unsafe { asm!("sti") };
    }

    fn window_mut(&self) -> Option<&'static mut Window> {
        layer_manager()
            .get_layer_mut(self.layer_id)
            .map(|l| l.get_window_mut())
    }

    pub fn can_write_on_this_line(&self) -> bool {
        self.cursor.x < COLUMNS as i32 - 1
    }

    pub fn back_space(&mut self) {
        self.cursor.x -= 1;
        if let Some(window) = self.window_mut() {
            fill_rectangle(
                &mut window.normal_window_writer(),
                &self.calc_cursor_pos(),
                &Vector2D::new(8, 16),
                &COLOR_BLACK,
            );
        }
    }

    pub fn input_ascii(&mut self, ascii: char) {
        assert!(self.can_write_on_this_line());

        let pos = self.calc_cursor_pos();
        if let Some(window) = self.window_mut() {
            write_ascii(
                &mut window.normal_window_writer(),
                pos.x,
                pos.y,
                ascii,
                &COLOR_WHITE,
            );
        }
        self.cursor.x += 1;
    }

    pub fn clear(&mut self) {
        if let Some(window) = self.window_mut() {
            fill_rectangle(
                window,
                &Vector2D::new(4, 4),
                &Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                &COLOR_BLACK,
            );
        }
        self.cursor = Vector2D::new(0, 0);
    }

    pub fn history_up_down(&mut self, line: &str) -> Rectangle<i32> {
        self.cursor.x = 1;
        let first_pos = self.calc_cursor_pos();
        let draw_area = Rectangle::new(first_pos, Vector2D::new(8 * (COLUMNS as i32 - 1), 16));
        if let Some(window) = self.window_mut() {
            fill_rectangle(
                &mut window.normal_window_writer(),
                &draw_area.pos,
                &draw_area.size,
                &COLOR_BLACK,
            );
        }

        if let Some(window) = self.window_mut() {
            write_string(
                &mut window.normal_window_writer(),
                first_pos.x,
                first_pos.y,
                line,
                &COLOR_WHITE,
            );
        }
        self.cursor.x = line.len() as i32 + 1;

        draw_area
    }
}

impl Write for TerminalWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.print(s);
        Ok(())
    }
}
