use crate::frame_buffer::FrameBuffer;
use crate::graphics::{Rectangle, Vector2D};
use crate::message::{LayerMessage, LayerOperation};
use crate::window::Window;
use alloc::vec;
use alloc::vec::Vec;
use shared::FrameBufferConfig;

pub mod global {
    use super::LayerManager;
    use crate::console::global::console;
    use crate::console::new_console_window;
    use crate::console::Mode::ConsoleWindow;
    use crate::frame_buffer::FrameBuffer;
    use crate::graphics::global::{frame_buffer_config, screen_size};
    use crate::graphics::{draw_desktop, Vector2D};
    use crate::Window;

    static mut SCREEN_FRAME_BUFFER: Option<FrameBuffer> = None;
    pub fn screen_frame_buffer() -> &'static mut FrameBuffer {
        unsafe { SCREEN_FRAME_BUFFER.as_mut().unwrap() }
    }

    static mut LAYER_MANAGER: Option<LayerManager> = None;
    pub fn layer_manager_op() -> Option<&'static mut LayerManager<'static>> {
        unsafe { LAYER_MANAGER.as_mut() }
    }
    pub fn layer_manager() -> &'static mut LayerManager<'static> {
        unsafe { LAYER_MANAGER.as_mut().unwrap() }
    }

    static mut BG_WINDOW: Option<Window> = None;
    pub fn bg_window() -> &'static mut Window {
        unsafe { BG_WINDOW.as_mut().unwrap() }
    }
    pub fn bg_window_ref() -> &'static Window {
        unsafe { BG_WINDOW.as_ref().unwrap() }
    }

    static mut CONSOLE_WINDOW: Option<Window> = None;
    pub fn console_window() -> &'static mut Window {
        unsafe { CONSOLE_WINDOW.as_mut().unwrap() }
    }
    pub fn console_window_ref() -> &'static Window {
        unsafe { CONSOLE_WINDOW.as_ref().unwrap() }
    }

    pub fn initialize() {
        let screen_size = screen_size();
        unsafe {
            BG_WINDOW = Some(Window::new(
                screen_size.x,
                screen_size.y,
                frame_buffer_config().pixel_format,
            ))
        }
        draw_desktop(bg_window().writer());

        unsafe {
            SCREEN_FRAME_BUFFER = Some(FrameBuffer::new(*frame_buffer_config()));
            LAYER_MANAGER = Some(LayerManager::new(frame_buffer_config()));
            CONSOLE_WINDOW = Some(new_console_window(frame_buffer_config().pixel_format));
        };
        console().reset_mode(ConsoleWindow, console_window());

        let bg_layer_id = layer_manager()
            .new_layer()
            .set_window(bg_window_ref())
            .move_(Vector2D::new(0, 0))
            .id();
        console().set_layer_id(
            layer_manager()
                .new_layer()
                .set_window(console_window_ref())
                .move_(Vector2D::new(0, 0))
                .id(),
        );

        layer_manager().up_down(bg_layer_id, 0);
        layer_manager().up_down(console().layer_id().unwrap(), 1);
    }
}

pub struct Layer<'a> {
    id: u32,
    position: Vector2D<i32>,
    window: Option<&'a Window>,
    draggable: bool,
}

impl<'a> Layer<'a> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            position: Vector2D::new(0, 0),
            window: None,
            draggable: false,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn set_window(&mut self, window: &'a Window) -> &mut Layer<'a> {
        self.window = Some(window);
        self
    }

    pub fn get_window(&self) -> Option<&'a Window> {
        self.window
    }

    pub fn set_draggable(&mut self, draggable: bool) -> &mut Layer<'a> {
        self.draggable = draggable;
        self
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub fn move_(&mut self, pos: Vector2D<i32>) -> &mut Layer<'a> {
        self.position = pos;
        self
    }

    pub fn move_relative(&mut self, diff: Vector2D<i32>) {
        let x = self.position.x + diff.x;
        let y = self.position.y + diff.y;
        self.position = Vector2D::new(x, y)
    }

    fn draw_to(&self, screen: &mut FrameBuffer, area: Rectangle<i32>) {
        if let Some(w) = self.window {
            w.draw_to(screen, self.position, area)
        }
    }
}

pub struct LayerManager<'a> {
    layers: Vec<Layer<'a>>,
    layer_id_stack: Vec<u32>,
    latest_id: u32,
    back_buffer: FrameBuffer,
}

impl<'a> LayerManager<'a> {
    pub fn new(screen_buffer_config: &FrameBufferConfig) -> LayerManager<'a> {
        let back_buffer = FrameBuffer::new(FrameBufferConfig::new(
            screen_buffer_config.horizontal_resolution,
            screen_buffer_config.vertical_resolution,
            screen_buffer_config.pixels_per_scan_line,
            screen_buffer_config.pixel_format,
        ));

        Self {
            layers: vec![],
            layer_id_stack: vec![],
            latest_id: 0,
            back_buffer,
        }
    }

    pub fn new_layer(&mut self) -> &mut Layer<'a> {
        self.layers.push(Layer::new(self.latest_id));
        self.latest_id += 1; // increment after layer.push to make layer_id and index of layers equal
        self.layers.iter_mut().last().unwrap()
    }

    pub fn draw_on(&mut self, area: Rectangle<i32>, screen: &mut FrameBuffer) {
        for &layer_id in &self.layer_id_stack {
            let index = layer_id as usize;
            self.layers[index].draw_to(&mut self.back_buffer, area);
        }
        screen.copy(area.pos, &self.back_buffer, area);
    }

    pub fn draw_layer_of(&mut self, id: u32, screen: &mut FrameBuffer) {
        let mut draw = false;
        let mut window_area: Rectangle<i32> = Rectangle::default();
        for &layer_id in &self.layer_id_stack {
            let index = layer_id as usize;
            let layer = &self.layers[index];

            if layer_id == id {
                window_area.size = layer.window.unwrap().size().to_i32_vec2d();
                window_area.pos = layer.position;
                draw = true
            }

            if draw {
                layer.draw_to(&mut self.back_buffer, window_area);
            }
        }
        screen.copy(window_area.pos, &self.back_buffer, window_area);
    }

    pub fn move_(&mut self, id: u32, new_position: Vector2D<i32>, screen: &mut FrameBuffer) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            let window_size = layer.get_window().unwrap().size();
            let old_pos = layer.position;
            layer.move_(new_position);
            self.draw_on(Rectangle::new(old_pos, window_size.to_i32_vec2d()), screen);
            self.draw_layer_of(id, screen);
        }
    }

    pub fn move_relative(&mut self, id: u32, pos_diff: Vector2D<i32>, screen: &mut FrameBuffer) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            let window_size = layer.get_window().unwrap().size();
            let old_pos = layer.position;
            layer.move_relative(pos_diff);
            self.draw_on(Rectangle::new(old_pos, window_size.to_i32_vec2d()), screen);
            self.draw_layer_of(id, screen);
        }
    }

    pub fn up_down(&mut self, id: u32, new_height: i32) {
        if self.layers.is_empty() {
            return;
        }

        if new_height.is_negative() {
            self.hide(id);
            return;
        }

        let new_height = {
            let h = new_height as usize;
            if h > self.layer_id_stack.len() {
                self.layer_id_stack.len()
            } else {
                h
            }
        };

        let showing_layer_id = self
            .layer_id_stack
            .iter()
            .enumerate()
            .find(|(_, &layer_id)| layer_id == id);
        match showing_layer_id {
            None => {
                // in case of the layer doesn't show yet
                self.layers.iter().find(|l| l.id == id).unwrap(); // check the layer exists
                self.layer_id_stack.insert(new_height, id);
            }
            Some((old_index, &layer_id)) => {
                let height = if new_height == self.layer_id_stack.len() - 1 {
                    new_height - 1 // decrement because the stack will remove
                } else {
                    new_height
                };
                self.layer_id_stack.remove(old_index);
                self.layer_id_stack.insert(height - 1, layer_id);
            }
        }
    }

    //TODO: remove the lifetime annotation of self after Layer changes not to have a Window reference
    pub fn find_layer_by_position(
        &'a self,
        pos: Vector2D<i32>,
        exclude_id: u32,
    ) -> Option<&Layer<'a>> {
        self.layer_id_stack
            .iter()
            .rev()
            .filter(|&&id| id != exclude_id)
            .map(|&id| &self.layers[id as usize])
            .find(|&layer| {
                if let Some(win) = layer.get_window() {
                    let win_pos = layer.position;
                    let win_end_pos = win_pos + win.size().to_i32_vec2d();
                    win_pos.x <= pos.x
                        && pos.x < win_end_pos.x
                        && win_pos.y <= pos.y
                        && pos.y < win_end_pos.y
                } else {
                    false
                }
            })
    }

    pub fn process_message(&mut self, message: &LayerMessage, screen: &mut FrameBuffer) {
        match message.op {
            LayerOperation::Move { pos } => self.move_(message.layer_id, pos, screen),
            LayerOperation::MoveRelative { pos } => {
                self.move_relative(message.layer_id, pos, screen)
            }
            LayerOperation::Draw => self.draw_layer_of(message.layer_id, screen),
        }
    }

    fn hide(&mut self, id: u32) {
        if self.layers.is_empty() {
            return;
        }

        let last_id = *self.layer_id_stack.last().unwrap();
        let hiding_index = self
            .layers
            .iter()
            .position(|l| l.id == id && l.id != last_id);

        if let Some(i) = hiding_index {
            self.layer_id_stack.remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{FrameBufferConfig, PixelFormat};

    #[test]
    fn new_layer() {
        let window1 = Window::new(1, 1, PixelFormat::KPixelBGRResv8BitPerColor);
        let mut lm = LayerManager::new(&FrameBufferConfig::new(
            1,
            1,
            1,
            PixelFormat::KPixelBGRResv8BitPerColor,
        ));
        let id1 = lm.new_layer().set_window(&window1).id();
        // verify layer's id equals to index of lm.layers
        assert_eq!(lm.layers[id1 as usize].id, id1);
    }

    #[test]
    fn move_() {
        let window1 = Window::new(1, 1, PixelFormat::KPixelBGRResv8BitPerColor);
        let mut lm = LayerManager::new(&frame_buffer_config());
        let id1 = lm.new_layer().set_window(&window1).id();

        lm.move_(
            id1,
            Vector2D::new(100, 10),
            &mut FrameBuffer::new(frame_buffer_config()),
        );

        let l1 = lm.layers.iter().find(|l| l.id == id1).unwrap();
        assert_eq!(l1.position, Vector2D::new(100, 10));
    }

    #[test]
    fn move_relative() {
        let window1 = Window::new(1, 1, PixelFormat::KPixelBGRResv8BitPerColor);
        let mut lm = LayerManager::new(&frame_buffer_config());
        let mut buffer = FrameBuffer::new(frame_buffer_config());
        let id1 = lm
            .new_layer()
            .set_window(&window1)
            .move_(Vector2D::new(100, 100))
            .id();

        lm.move_relative(id1, Vector2D::new(-50, -30), &mut buffer);
        {
            let l1 = lm.layers.iter().find(|l| l.id == id1).unwrap();
            assert_eq!(l1.position, Vector2D::new(50, 70));
        }

        lm.move_relative(id1, Vector2D::new(-60, -60), &mut buffer);
        let l1 = lm.layers.iter().find(|l| l.id == id1).unwrap();
        assert_eq!(l1.position, Vector2D::new(-10, 10));
    }

    #[test]
    fn up_down() {
        let mut lm = LayerManager::new(&FrameBufferConfig::new(
            1,
            1,
            1,
            PixelFormat::KPixelBGRResv8BitPerColor,
        ));
        let id0 = lm.new_layer().id;
        let id1 = lm.new_layer().id;
        let id2 = lm.new_layer().id;
        let id3 = lm.new_layer().id;

        lm.up_down(id0, 0);
        lm.up_down(id1, 100);
        assert_eq!(vec![id0, id1], lm.layer_id_stack);

        lm.up_down(id2, 0);
        assert_eq!(vec![id2, id0, id1], lm.layer_id_stack);

        lm.up_down(id3, 100);
        assert_eq!(vec![id2, id0, id1, id3], lm.layer_id_stack);

        lm.up_down(id0, i32::MAX);
        assert_eq!(vec![id2, id1, id3, id0], lm.layer_id_stack);

        lm.up_down(id1, -1);
        assert_eq!(vec![id2, id3, id0], lm.layer_id_stack);
    }

    fn frame_buffer_config() -> FrameBufferConfig {
        FrameBufferConfig::new(1, 1, 1, PixelFormat::KPixelBGRResv8BitPerColor)
    }
}
