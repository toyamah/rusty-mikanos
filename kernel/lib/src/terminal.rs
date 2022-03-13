use crate::font::{write_ascii, write_string};
use crate::graphics::{
    draw_text_box_with_colors, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D,
    COLOR_BLACK, COLOR_WHITE,
};
use crate::layer::LayerManager;
use crate::window::TITLED_WINDOW_TOP_LEFT_MARGIN;
use crate::Window;
use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
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
    command_history: CommandHistory,
}

impl Terminal {
    fn new() -> Terminal {
        Self {
            layer_id: u32::MAX,
            cursor: Vector2D::new(0, 0),
            is_cursor_visible: false,
            line_buf: String::with_capacity(LINE_MAX),
            command_history: CommandHistory::new(),
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
        keycode: u8,
        ascii: char,
        window: &mut Window,
    ) -> Rectangle<i32> {
        self.draw_cursor(window, false);

        let mut draw_area = Rectangle::new(self.calc_cursor_pos(), Vector2D::new(8 * 2, 16));

        match ascii {
            '\n' => {
                self.command_history.push(self.line_buf.to_string());

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
            '\x00' => {
                if keycode == 0x51 {
                    draw_area = self.history_up_down(Direction::Down, window);
                } else if keycode == 0x52 {
                    draw_area = self.history_up_down(Direction::Up, window);
                }
            }
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
            "clear" => {
                fill_rectangle(
                    w,
                    &Vector2D::new(4, 4),
                    &Vector2D::new(8 * COLUMNS as i32, 16 * ROWS as i32),
                    &COLOR_BLACK,
                );
                self.cursor = Vector2D::new(0, 0);
            }
            "lspci" => {
                // comment out because of referencing a global variable
                // for device in devices() {
                //     self.print(format!("{}\n", device).as_str(), w);
                // }
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

    fn history_up_down(&mut self, direction: Direction, w: &mut Window) -> Rectangle<i32> {
        self.cursor.x = 1;
        let first_pos = self.calc_cursor_pos();
        let draw_area = Rectangle::new(first_pos, Vector2D::new(8 * (COLUMNS as i32 - 1), 16));
        fill_rectangle(
            &mut w.normal_window_writer(),
            &draw_area.pos,
            &draw_area.size,
            &COLOR_BLACK,
        );

        self.line_buf = match direction {
            Direction::Up => self.command_history.up().to_string(),
            Direction::Down => self.command_history.down().to_string(),
        };
        write_string(
            &mut w.normal_window_writer(),
            first_pos.x,
            first_pos.y,
            self.line_buf.as_str(),
            &COLOR_WHITE,
        );
        self.cursor.x = self.line_buf.len() as i32 + 1;

        draw_area
    }
}

enum Direction {
    Up,
    Down,
}

#[derive(Debug)]
struct CommandHistory {
    //       New      Old
    // index   0 1 2 3
    history: VecDeque<String>,
    pointing_index: Option<usize>,
}

impl CommandHistory {
    const MAX: usize = 8;

    fn new() -> CommandHistory {
        Self {
            history: VecDeque::with_capacity(Self::MAX),
            pointing_index: None,
        }
    }

    fn up(&mut self) -> &str {
        if self.history.is_empty() {
            self.pointing_index = None;
            return "";
        }

        self.pointing_index = match self.pointing_index {
            None => Some(0), // return the newest
            Some(i) if i + 1 < self.history.len() => Some(i + 1),
            Some(_) => Some(self.history.len() - 1), // return the oldest
        };
        self.pointing_index
            .map(|i| self.history.get(i).unwrap())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn down(&mut self) -> &str {
        self.pointing_index = match self.pointing_index {
            Some(i) if i > 0 => Some(i - 1),
            Some(_) => None, // return None because of no more new command
            None => None,
        };

        self.pointing_index
            .map(|i| self.history.get(i).unwrap())
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn push(&mut self, command: String) {
        self.pointing_index = None;

        if command.is_empty() {
            return;
        }

        if self.history.len() == Self::MAX {
            self.history.pop_back().unwrap();
        }
        self.history.push_front(command);
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
mod command_history_tests {
    use crate::terminal::CommandHistory;
    use alloc::string::ToString;

    #[test]
    fn up_should_return_empty_if_it_has_no_history() {
        let mut history = CommandHistory::new();
        assert_eq!(history.up(), "");
    }

    #[test]
    fn up_should_return_next_old_comand_if_it_has_history() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        assert_eq!(history.up(), "c");
        assert_eq!(history.up(), "b");
        assert_eq!(history.up(), "a");
        assert_eq!(history.up(), "a");
        assert_eq!(history.up(), "a");
    }

    #[test]
    fn down_should_return_empty_if_it_has_no_history() {
        let mut history = CommandHistory::new();
        assert_eq!(history.down(), "");
    }

    #[test]
    fn down_should_return_next_new_command_if_it_has_history() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        history.up(); // c
        history.up(); // b
        history.up(); // a
        history.up(); // a and pointing index should not be changed.

        assert_eq!(history.down(), "b");
        assert_eq!(history.down(), "c");
        assert_eq!(history.down(), "");
        assert_eq!(history.down(), "");
        assert_eq!(history.down(), "");
    }

    #[test]
    fn push_should_reset_index() {
        let mut history = CommandHistory::new();
        history.push("a".to_string());
        history.push("b".to_string());
        history.push("c".to_string());

        history.up(); // c
        history.up(); // b
        history.up(); // a

        history.push("d".to_string());

        // up should return the newest command because of resetting the index.
        assert_eq!(history.up(), "d")
    }

    #[test]
    fn push_should_remove_oldest_if_history_is_full() {
        let mut history = CommandHistory::new();
        for i in 0..CommandHistory::MAX {
            history.push(i.to_string());
        }

        history.push(CommandHistory::MAX.to_string());

        assert_eq!(
            history.history.front().unwrap(),
            &CommandHistory::MAX.to_string()
        );
        assert_eq!(history.history.back().unwrap(), &"1".to_string()); // not "0"
    }
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
