use crate::font::write_ascii;
use crate::graphics::{
    draw_text_box_with_colors, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D,
    COLOR_BLACK, COLOR_WHITE,
};
use crate::layer::LayerManager;
use crate::window::TITLED_WINDOW_TOP_LEFT_MARGIN;
use crate::Window;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem;
use shared::PixelFormat;

pub mod global {
    use crate::graphics::global::frame_buffer_config;
    use crate::graphics::Vector2D;
    use crate::layer::global::{active_layer, layer_manager, layer_task_map, screen_frame_buffer};
    use crate::message::{LayerMessage, LayerOperation, Message, MessageType};
    use crate::task::global::task_manager;
    use crate::terminal::Terminal;
    use crate::Window;
    use core::arch::asm;

    pub fn task_terminal(task_id: u64, _: usize) {
        unsafe { asm!("cli") };
        let current_task_id = task_manager().current_task().id();
        let mut terminal = Terminal::new();
        terminal.initialize(layer_manager(), frame_buffer_config().pixel_format);
        layer_manager().move_(
            terminal.layer_id,
            Vector2D::new(100, 200),
            screen_frame_buffer(),
        );
        active_layer().activate(
            Some(terminal.layer_id),
            layer_manager(),
            screen_frame_buffer(),
        );
        layer_task_map().insert(terminal.layer_id, task_id);
        unsafe { asm!("sti") };

        loop {
            unsafe { asm!("cli") };
            let msg = task_manager()
                .get_task_mut(current_task_id)
                .unwrap()
                .receive_message();
            if msg.is_none() {
                task_manager().sleep(current_task_id).unwrap();
                unsafe { asm!("sti") };
                continue;
            };

            let msg = msg.unwrap();
            match msg.m_type {
                MessageType::InterruptXhci => {}
                MessageType::TimerTimeout {
                    timeout: _,
                    value: _,
                } => {
                    let area = terminal.blink_cursor(terminal_window(terminal.layer_id));

                    let msg = Message::new(MessageType::Layer(LayerMessage {
                        layer_id: terminal.layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager()
                        .send_message(task_manager().main_task().id(), msg)
                        .unwrap();
                    unsafe { asm!("sti") };
                }
                MessageType::KeyPush {
                    modifier,
                    keycode,
                    ascii,
                } => {
                    let area = terminal.input_key(
                        modifier,
                        keycode,
                        ascii,
                        terminal_window(terminal.layer_id),
                    );
                    let msg = Message::new(MessageType::Layer(LayerMessage {
                        layer_id: terminal.layer_id,
                        op: LayerOperation::DrawArea(area),
                        src_task_id: task_id,
                    }));
                    unsafe { asm!("cli") };
                    task_manager()
                        .send_message(task_manager().main_task().id(), msg)
                        .unwrap();
                    unsafe { asm!("sti") };
                }
                MessageType::Layer(_) => {}
                MessageType::LayerFinish => {}
            }
        }
    }

    fn terminal_window(terminal_layer_id: u32) -> &'static mut Window {
        layer_manager()
            .get_layer_mut(terminal_layer_id)
            .expect("couldn't find terminal window")
            .get_window_mut()
    }
}

const ROWS: usize = 15;
const COLUMNS: usize = 60;
const LINE_MAX: usize = 128;

struct Terminal {
    layer_id: u32,
    cursor: Vector2D<i32>,
    is_cursor_visible: bool,
    line_buf: String,
}

impl Terminal {
    fn new() -> Terminal {
        Self {
            layer_id: u32::MAX,
            cursor: Vector2D::new(0, 0),
            is_cursor_visible: false,
            line_buf: String::with_capacity(LINE_MAX),
        }
    }

    fn initialize(&mut self, layout_manager: &mut LayerManager, pixel_format: PixelFormat) {
        let mut window = Window::new_with_title(
            COLUMNS * 8 + 8 + Window::TITLED_WINDOW_MARGIN.x as usize,
            ROWS * 16 + 8 + Window::TITLED_WINDOW_MARGIN.y as usize,
            pixel_format,
            "MikanTerm",
        );

        let inner_size = window.inner_size();
        draw_terminal(&mut window, Vector2D::new(0, 0), inner_size);
        self.print(">", &mut window);
        self.layer_id = layout_manager.new_layer(window).set_draggable(true).id();
    }

    fn blink_cursor(&mut self, window: &mut Window) -> Rectangle<i32> {
        self.is_cursor_visible = !self.is_cursor_visible;
        self.draw_cursor(window, self.is_cursor_visible);
        Rectangle::new(self.calc_cursor_pos(), Vector2D::new(7, 15))
    }

    fn draw_cursor(&mut self, window: &mut Window, visible: bool) {
        let color = if visible { &COLOR_WHITE } else { &COLOR_BLACK };
        fill_rectangle(
            &mut window.normal_window_writer(),
            &self.calc_cursor_pos(),
            &Vector2D::new(7, 15),
            color,
        );
    }

    fn input_key(
        &mut self,
        _modifier: u8,
        _keycode: u8,
        ascii: char,
        window: &mut Window,
    ) -> Rectangle<i32> {
        self.draw_cursor(window, false);

        let mut draw_area = Rectangle::new(self.calc_cursor_pos(), Vector2D::new(8 * 2, 16));

        match ascii {
            '\n' => {
                self.cursor.x = 0;
                if self.cursor.y < ROWS as i32 - 1 {
                    self.cursor.y += 1;
                } else {
                    self.scroll1(window);
                }
                self.execute_line(window);
                self.print(">", window);
                draw_area.pos = TITLED_WINDOW_TOP_LEFT_MARGIN;
                draw_area.size = window.inner_size();
            }
            '\x08' => {
                if self.line_buf.pop().is_some() {
                    self.cursor.x -= 1;
                    fill_rectangle(
                        &mut window.normal_window_writer(),
                        &self.calc_cursor_pos(),
                        &Vector2D::new(8, 16),
                        &COLOR_BLACK,
                    );
                    draw_area.pos = self.calc_cursor_pos();
                }
            }
            '\x00' => {}
            _ => {
                if self.cursor.x < COLUMNS as i32 - 1 && self.line_buf.len() < LINE_MAX {
                    self.line_buf.push(ascii);
                    let pos = self.calc_cursor_pos();
                    write_ascii(
                        &mut window.normal_window_writer(),
                        pos.x,
                        pos.y,
                        ascii,
                        &COLOR_WHITE,
                    );
                    self.cursor.x += 1;
                }
            }
        }

        self.draw_cursor(window, true);
        draw_area
    }

    fn calc_cursor_pos(&self) -> Vector2D<i32> {
        TITLED_WINDOW_TOP_LEFT_MARGIN + Vector2D::new(4 + 8 * self.cursor.x, 4 + 16 * self.cursor.y)
    }

    fn scroll1(&mut self, window: &mut Window) {
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

    fn execute_line(&mut self, w: &mut Window) {
        let line_buf = mem::take(&mut self.line_buf);
        let command = parse_command(line_buf.as_str());
        if command.is_none() {
            return;
        }
        let (command, args) = command.unwrap();

        match command {
            "echo" => {
                if let Some(&arg) = args.get(0) {
                    self.print(arg, w);
                }
                self.print("\n", w);
            }
            _ => {
                self.print("no such command: ", w);
                self.print(command, w);
                self.print("\n", w);
            }
        }
    }

    fn print(&mut self, s: &str, w: &mut Window) {
        self.draw_cursor(w, false);

        for char in s.chars() {
            match char {
                '\n' => self.new_line(w),
                _ => {
                    let pos = self.calc_cursor_pos();
                    write_ascii(
                        &mut w.normal_window_writer(),
                        pos.x,
                        pos.y,
                        char,
                        &COLOR_WHITE,
                    );
                    if self.cursor.x == COLUMNS as i32 - 1 {
                        self.new_line(w);
                    } else {
                        self.cursor.x += 1;
                    }
                }
            }
        }

        self.draw_cursor(w, false);
    }

    fn new_line(&mut self, w: &mut Window) {
        self.cursor.x = 0;
        if self.cursor.y < ROWS as i32 - 1 {
            self.cursor.y += 1;
        } else {
            self.scroll1(w)
        }
    }
}

fn draw_terminal<W: PixelWriter>(w: &mut W, pos: Vector2D<i32>, size: Vector2D<i32>) {
    draw_text_box_with_colors(
        w,
        pos,
        size,
        &COLOR_BLACK,
        &PixelColor::from(0xc6c6c6),
        &PixelColor::from(0x848484),
    );
}

fn parse_command(s: &str) -> Option<(&str, Vec<&str>)> {
    let mut parsed = s.trim().split_whitespace().collect::<VecDeque<_>>();
    if parsed.is_empty() {
        return None;
    }

    let command = parsed.pop_front().unwrap();
    Some((command, Vec::from(parsed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn parse_command_empty() {
        assert_eq!(parse_command(""), None);
    }

    #[test]
    fn parse_command_no_args() {
        assert_eq!(parse_command("echo"), Some(("echo", Vec::new())));
    }

    #[test]
    fn parse_command_one_arg() {
        assert_eq!(parse_command("echo a\\aa"), Some(("echo", vec!["a\\aa"])));
    }

    #[test]
    fn parse_command_args() {
        assert_eq!(
            parse_command("ls -l | sort"),
            Some(("ls", vec!["-l", "|", "sort"]))
        );
    }
}
