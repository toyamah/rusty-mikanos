use crate::frame_buffer::FrameBuffer;
use crate::graphics::{PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE};
use crate::layer::{ActiveLayer, LayerManager};
use crate::Window;
use shared::PixelFormat;

pub mod global {
    use super::{draw_mouse_cursor, new_mouse_cursor_window, Mouse};
    use crate::graphics::global::frame_buffer_config;
    use crate::graphics::Vector2D;
    use crate::layer::global::{
        active_layer, get_layer_window_mut, get_layer_window_ref, layer_manager,
        screen_frame_buffer,
    };
    use crate::Window;

    fn mouse_cursor_window() -> &'static mut Window {
        get_layer_window_mut(mouse().layer_id).expect("could not find mouse layer")
    }

    fn mouse_cursor_window_ref() -> &'static Window {
        get_layer_window_ref(mouse().layer_id).expect("could not find mouse layer")
    }

    static mut MOUSE: Option<Mouse> = None;
    pub fn mouse() -> &'static mut Mouse {
        unsafe { MOUSE.as_mut().unwrap() }
    }

    pub fn initialize() {
        let mut window = new_mouse_cursor_window(frame_buffer_config().pixel_format);
        draw_mouse_cursor(window.writer(), &Vector2D::new(0, 0));

        let mouse_layer_id = layer_manager().new_layer(window).id();

        unsafe { MOUSE = Some(Mouse::new(mouse_layer_id)) };
        mouse().set_position(
            Vector2D::new(200, 200),
            layer_manager(),
            screen_frame_buffer(),
        );
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
    layer_id: u32,
    position: Vector2D<i32>,
    drag_layer_id: Option<u32>,
    previous_buttons: u8,
}

impl Mouse {
    pub fn new(layer_id: u32) -> Mouse {
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
        screen_frame_buffer: &mut FrameBuffer,
        active_layer: &mut ActiveLayer,
    ) {
        let new_pos = self.position + Vector2D::new(displacement_x as i32, displacement_y as i32);
        let new_pos = new_pos
            .element_min(screen_size + Vector2D::new(-1, -1))
            .element_max(Vector2D::new(0, 0));

        let old_pos = self.position;
        self.position = new_pos;
        let pos_diff = self.position - old_pos;
        layout_manager.move_(self.layer_id, self.position, screen_frame_buffer);

        let previous_left_pressed = (self.previous_buttons & 0x01) != 0;
        let left_pressed = (buttons & 0x01) != 0;
        if !previous_left_pressed && left_pressed {
            let draggable_layer_id = layout_manager
                .find_layer_by_position(new_pos, self.layer_id)
                .filter(|l| l.is_draggable())
                .map(|l| l.id());
            if let Some(id) = draggable_layer_id {
                self.drag_layer_id = Some(id);
            }
            active_layer.activate(draggable_layer_id, layout_manager, screen_frame_buffer);
        } else if previous_left_pressed && left_pressed {
            if let Some(drag_layer_id) = self.drag_layer_id {
                layout_manager.move_relative(drag_layer_id, pos_diff, screen_frame_buffer);
            }
        } else if previous_left_pressed && !left_pressed {
            self.drag_layer_id = None;
        }

        self.previous_buttons = buttons;
    }
}

// don't know why Rust cannot compile this signature
// pub fn draw_mouse_cursor<W: PixelWriter>(writer: &W, position: &Vector2D<i32>) {
pub fn draw_mouse_cursor(writer: &mut Window, position: &Vector2D<i32>) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in row.chars().enumerate() {
            let color = match char {
                '@' => &COLOR_WHITE,
                '.' => &COLOR_BLACK,
                _ => &MOUSE_TRANSPARENT_COLOR,
            };
            writer.write(position.x + dx as i32, position.y + dy as i32, color);
        }
    }
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
