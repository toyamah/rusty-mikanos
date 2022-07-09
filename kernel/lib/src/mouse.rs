use crate::frame_buffer::FrameBuffer;
use crate::graphics::{PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE};
use crate::layer::{ActiveLayer, LayerID, LayerManager};
use crate::message::{
    Message, MessageType, MouseButtonMessage, MouseMoveMessage, WindowCloseMessage,
};
use crate::task::{TaskID, TaskManager};
use crate::window::WindowRegion;
use crate::Window;
use alloc::collections::BTreeMap;
use shared::PixelFormat;

pub mod global {
    use super::{draw_mouse_cursor, new_mouse_cursor_window, Mouse};
    use crate::graphics::global::frame_buffer_config;
    use crate::graphics::Vector2D;
    use crate::layer::global::{active_layer, layer_manager, screen_frame_buffer};
    use spin::Mutex;

    pub static MOUSE: Mutex<Option<Mouse>> = Mutex::new(None);

    pub fn initialize() {
        let mut window = new_mouse_cursor_window(frame_buffer_config().pixel_format);
        draw_mouse_cursor(window.writer(), &Vector2D::new(0, 0));

        let mouse_layer_id = layer_manager().new_layer(window).id();

        let mut mouse = Mouse::new(mouse_layer_id);
        mouse.set_position(
            Vector2D::new(200, 200),
            layer_manager(),
            screen_frame_buffer(),
        );
        *MOUSE.lock() = Some(mouse);

        layer_manager().up_down(mouse_layer_id, i32::MAX);
        active_layer().mouser_layer_id = mouse_layer_id;
    }
}

const MOUSE_TRANSPARENT_COLOR: PixelColor = PixelColor::new(0, 0, 1);
const MOUSE_CURSOR_SHAPE: [&str; 24] = [
    "@              ",
    "@@             ",
    "@.@            ",
    "@..@           ",
    "@...@          ",
    "@....@         ",
    "@.....@        ",
    "@......@       ",
    "@.......@      ",
    "@........@     ",
    "@.........@    ",
    "@..........@   ",
    "@...........@  ",
    "@............@ ",
    "@......@@@@@@@@",
    "@......@       ",
    "@....@@.@      ",
    "@...@ @.@      ",
    "@..@   @.@     ",
    "@.@    @.@     ",
    "@@      @.@    ",
    "@       @.@    ",
    "         @.@   ",
    "         @@@   ",
];

pub struct Mouse {
    layer_id: LayerID,
    position: Vector2D<i32>,
    drag_layer_id: Option<LayerID>,
    previous_buttons: u8,
}

impl Mouse {
    pub fn new(layer_id: LayerID) -> Mouse {
        Mouse {
            layer_id,
            position: Vector2D::new(0, 0),
            drag_layer_id: None,
            previous_buttons: 0,
        }
    }

    fn set_position(
        &mut self,
        position: Vector2D<i32>,
        layout_manager: &mut LayerManager,
        screen_buffer: &mut FrameBuffer,
    ) {
        self.position = position;
        layout_manager.move_(self.layer_id, self.position, screen_buffer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn on_interrupt(
        &mut self,
        buttons: u8,
        displacement_x: i8,
        displacement_y: i8,
        screen_size: Vector2D<i32>,
        layout_manager: &mut LayerManager,
        frame_buffer: &mut FrameBuffer,
        active_layer: &mut ActiveLayer,
        layer_task_map: &mut BTreeMap<LayerID, TaskID>,
        task_manager: &mut TaskManager,
    ) {
        let new_pos = self.position + Vector2D::new(displacement_x as i32, displacement_y as i32);
        let new_pos = new_pos
            .element_min(screen_size + Vector2D::new(-1, -1))
            .element_max(Vector2D::new(0, 0));

        let old_pos = self.position;
        self.position = new_pos;
        let pos_diff = self.position - old_pos;
        layout_manager.move_(self.layer_id, self.position, frame_buffer);

        let mut close_layer_id = None;

        let previous_left_pressed = (self.previous_buttons & 0x01) != 0;
        let left_pressed = (buttons & 0x01) != 0;
        if !previous_left_pressed && left_pressed {
            let draggable_layer = layout_manager
                .find_layer_by_position(new_pos, self.layer_id)
                .filter(|l| l.is_draggable());
            if let Some(layer) = draggable_layer {
                match layer
                    .get_window_ref()
                    .get_window_region(self.position - layer.position())
                {
                    WindowRegion::TitleBar => self.drag_layer_id = Some(layer.id()),
                    WindowRegion::CloseButton => close_layer_id = Some(layer.id()),
                    WindowRegion::Border => {}
                    WindowRegion::Other => {}
                }
            }
            let draggable_id = draggable_layer.map(|l| l.id());
            active_layer.activate(
                draggable_id,
                layout_manager,
                frame_buffer,
                task_manager,
                layer_task_map,
            );
        } else if previous_left_pressed && left_pressed {
            if let Some(drag_layer_id) = self.drag_layer_id {
                layout_manager.move_relative(drag_layer_id, pos_diff, frame_buffer);
            }
        } else if previous_left_pressed && !left_pressed {
            self.drag_layer_id = None;
        }

        if self.drag_layer_id == None {
            if close_layer_id.is_some() {
                send_close_message(active_layer, layer_task_map, task_manager);
            } else {
                send_mouse_message(
                    new_pos,
                    pos_diff,
                    buttons,
                    self.previous_buttons,
                    layout_manager,
                    active_layer,
                    layer_task_map,
                    task_manager,
                );
            }
        }

        self.previous_buttons = buttons;
    }
}

pub fn draw_mouse_cursor<W: PixelWriter>(writer: &mut W, position: &Vector2D<i32>) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in row.chars().enumerate() {
            let color = match char {
                '@' => &COLOR_BLACK,
                '.' => &COLOR_WHITE,
                _ => &MOUSE_TRANSPARENT_COLOR,
            };
            writer.write(position.x + dx as i32, position.y + dy as i32, color);
        }
    }
}

fn find_active_layer_task(
    active_layer: &ActiveLayer,
    layer_task_map: &BTreeMap<LayerID, TaskID>,
) -> Option<(LayerID, TaskID)> {
    let layer_id = match active_layer.get_active_layer_id() {
        None => return None,
        Some(id) => id,
    };
    let task_id = match layer_task_map.get(&layer_id) {
        None => return None,
        Some(&id) => id,
    };
    Some((layer_id, task_id))
}

#[allow(clippy::too_many_arguments)]
pub fn send_mouse_message(
    newpos: Vector2D<i32>,
    posdiff: Vector2D<i32>,
    buttons: u8,
    previous_buttons: u8,
    layer_manager: &LayerManager,
    active_layer: &mut ActiveLayer,
    layer_task_map: &mut BTreeMap<LayerID, TaskID>,
    task_manager: &mut TaskManager,
) {
    let (layer_id, task_id) = match find_active_layer_task(active_layer, layer_task_map) {
        None => return,
        Some(pair) => pair,
    };
    let layer = match layer_manager.get_layer(layer_id) {
        None => return,
        Some(l) => l,
    };

    let relpos = newpos - layer.position();
    if posdiff.x != 0 || posdiff.y != 0 {
        let relpos = newpos - layer.position();
        let msg = Message::new(MessageType::MouseMove(MouseMoveMessage {
            x: relpos.x,
            y: relpos.y,
            dx: posdiff.x,
            dy: posdiff.y,
            buttons,
        }));

        task_manager
            .send_message(task_id, msg)
            .expect("failed to send message");
    }

    if previous_buttons != buttons {
        let diff = previous_buttons ^ buttons;
        for i in 0..8 {
            let is_button_state_changed = ((diff >> i) & 1) == 1;
            if is_button_state_changed {
                let msg = Message::new(MessageType::MouseButton(MouseButtonMessage {
                    x: relpos.x,
                    y: relpos.y,
                    press: ((buttons >> i) & 1) as i32,
                    button: i,
                }));
                task_manager
                    .send_message(task_id, msg)
                    .expect("failed to send message");
            }
        }
    }
}

fn send_close_message(
    active_layer: &mut ActiveLayer,
    layer_task_map: &mut BTreeMap<LayerID, TaskID>,
    task_manager: &mut TaskManager,
) {
    let (layer_id, task_id) = match find_active_layer_task(active_layer, layer_task_map) {
        None => return,
        Some(pair) => pair,
    };

    let message = Message::new(MessageType::WindowClose(WindowCloseMessage { layer_id }));
    let _ = task_manager.send_message(task_id, message);
}

pub fn new_mouse_cursor_window(pixel_format: PixelFormat) -> Window {
    let mut window = Window::new(
        MOUSE_CURSOR_SHAPE[0].len(),
        MOUSE_CURSOR_SHAPE.len(),
        pixel_format,
    );
    window.set_transparent_color(MOUSE_TRANSPARENT_COLOR);
    window
}
