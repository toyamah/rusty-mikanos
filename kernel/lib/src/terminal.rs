use crate::graphics::{
    draw_text_box_with_colors, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D,
    COLOR_BLACK, COLOR_WHITE,
};
use crate::layer::LayerManager;
use crate::window::TITLED_WINDOW_TOP_LEFT_MARGIN;
use crate::Window;
use shared::PixelFormat;

pub mod global {
    use crate::graphics::global::frame_buffer_config;
    use crate::graphics::Vector2D;
    use crate::layer::global::{active_layer, layer_manager, screen_frame_buffer};
    use crate::layer::Layer;
    use crate::message::{LayerMessage, LayerOperation, Message, MessageType};
    use crate::task::global::task_manager;
    use crate::terminal::Terminal;
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
                    let area =
                        terminal.blink_cursor(terminal_window(terminal.layer_id).get_window_mut());

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
                MessageType::KeyPush { .. } => {}
                MessageType::Layer(_) => {}
                MessageType::LayerFinish => {}
            }
        }
    }

    fn terminal_window(terminal_layer_id: u32) -> &'static mut Layer {
        layer_manager()
            .get_layer_mut(terminal_layer_id)
            .expect("couldn't find terminal window")
    }
}

const ROWS: usize = 15;
const COLUMNS: usize = 60;

struct Terminal {
    layer_id: u32,
    cursor: Vector2D<i32>,
    is_cursor_visible: bool,
}

impl Terminal {
    fn new() -> Terminal {
        Self {
            layer_id: u32::MAX,
            cursor: Vector2D::new(0, 0),
            is_cursor_visible: false,
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
        self.layer_id = layout_manager.new_layer(window).set_draggable(true).id();
    }

    fn blink_cursor(&mut self, window: &mut Window) -> Rectangle<i32> {
        self.is_cursor_visible = !self.is_cursor_visible;
        self.draw_cursor(window, self.is_cursor_visible);

        Rectangle::new(
            TITLED_WINDOW_TOP_LEFT_MARGIN
                + Vector2D::new(4 + 8 * self.cursor.x, 5 + 16 * self.cursor.y),
            Vector2D::new(7, 15),
        )
    }

    fn draw_cursor(&mut self, window: &mut Window, visible: bool) {
        let color = if visible { &COLOR_BLACK } else { &COLOR_WHITE };
        let pos = Vector2D::new(4 + 8 * self.cursor.x, 5 + 16 * self.cursor.y);
        fill_rectangle(window, &pos, &Vector2D::new(7, 15), color);
    }
}

fn draw_terminal<W: PixelWriter>(w: &mut W, pos: Vector2D<i32>, size: Vector2D<i32>) {
    draw_text_box_with_colors(
        w,
        pos,
        size,
        &COLOR_WHITE,
        &PixelColor::from(0xc6c6c6),
        &PixelColor::from(0x848484),
    );
}
