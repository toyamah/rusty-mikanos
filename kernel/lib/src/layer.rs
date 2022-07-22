use crate::error::{Code, Error};
use crate::frame_buffer::FrameBuffer;
use crate::graphics::{Rectangle, Vector2D};
use crate::layer::global::screen_frame_buffer;
use crate::make_error;
use crate::message::{LayerMessage, LayerOperation, Message, MessageType, WindowActiveMode};
use crate::sync::{Mutex, MutexGuard};
use crate::task::global::task_manager;
use crate::task::TaskID;
use crate::window::Window;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use core::cmp;
use core::fmt::{Display, Formatter};
use core::ops::AddAssign;
use shared::FrameBufferConfig;

pub mod global {
    use super::LayerManager;
    use crate::console::global::console;
    use crate::console::new_console_window;
    use crate::console::Mode::ConsoleWindow;
    use crate::frame_buffer::FrameBuffer;
    use crate::graphics::global::{frame_buffer_config, screen_size};
    use crate::graphics::{draw_desktop, Vector2D};
    use crate::sync::Mutex;
    use crate::Window;
    use alloc::sync::Arc;
    use spin::Once;

    pub(super) static SCREEN_FRAME_BUFFER: Once<Mutex<FrameBuffer>> = Once::new();
    pub(super) fn screen_frame_buffer() -> &'static Mutex<FrameBuffer> {
        SCREEN_FRAME_BUFFER.call_once(|| Mutex::new(FrameBuffer::new(*frame_buffer_config())))
    }

    static LAYER_MANAGER: Once<Mutex<LayerManager>> = Once::new();
    pub fn layer_manager_op() -> Option<&'static Mutex<LayerManager>> {
        LAYER_MANAGER.get()
    }
    pub fn layer_manager() -> &'static Mutex<LayerManager> {
        LAYER_MANAGER.call_once(|| Mutex::new(LayerManager::new(frame_buffer_config())))
    }

    pub fn initialize() {
        let screen_size = screen_size();
        let mut bg_window = Window::new(
            screen_size.x,
            screen_size.y,
            frame_buffer_config().pixel_format,
        );
        draw_desktop(bg_window.writer());

        SCREEN_FRAME_BUFFER.call_once(|| Mutex::new(FrameBuffer::new(*frame_buffer_config())));

        let console_window = Arc::new(Mutex::new(new_console_window(
            frame_buffer_config().pixel_format,
        )));
        console().reset_mode(ConsoleWindow(Arc::clone(&console_window)));

        let mut layout_manager = layer_manager().lock();
        let bg_layer_id = layout_manager
            .new_layer(Arc::new(Mutex::new(bg_window)))
            .move_(Vector2D::new(0, 0))
            .id();
        console().set_layer_id(
            layout_manager
                .new_layer(console_window)
                .move_(Vector2D::new(0, 0))
                .id(),
        );

        layout_manager.up_down(bg_layer_id, 0);
        layout_manager.up_down(console().layer_id().unwrap(), 1);
    }
}

pub struct Layer {
    id: LayerID,
    position: Vector2D<i32>,
    window: Arc<Mutex<Window>>,
    draggable: bool,
}

impl Layer {
    pub fn new(id: LayerID, window: Arc<Mutex<Window>>) -> Self {
        Self {
            id,
            position: Vector2D::new(0, 0),
            window,
            draggable: false,
        }
    }

    pub fn id(&self) -> LayerID {
        self.id
    }

    pub fn position(&self) -> Vector2D<i32> {
        self.position
    }

    pub fn get_window_ref(&self) -> MutexGuard<Window> {
        self.window.lock()
    }

    pub fn get_window_mut(&mut self) -> MutexGuard<Window> {
        self.window.lock()
    }

    pub fn set_draggable(&mut self, draggable: bool) -> &mut Layer {
        self.draggable = draggable;
        self
    }

    pub fn is_draggable(&self) -> bool {
        self.draggable
    }

    pub fn move_(&mut self, pos: Vector2D<i32>) -> &mut Layer {
        self.position = pos;
        self
    }

    pub fn move_relative(&mut self, diff: Vector2D<i32>) {
        let x = self.position.x + diff.x;
        let y = self.position.y + diff.y;
        self.position = Vector2D::new(x, y)
    }

    fn draw_to(&mut self, screen: &mut FrameBuffer, area: Rectangle<i32>) {
        self.window.lock().draw_to(screen, self.position, area)
    }
}

pub struct LayerManager {
    layers: BTreeMap<LayerID, Layer>,
    layer_id_stack: Vec<LayerID>,
    latest_id: LayerID,
    back_buffer: FrameBuffer,
    active_layer: ActiveLayer,
    layer_task_map: BTreeMap<LayerID, TaskID>,
}

impl LayerManager {
    pub fn new(screen_buffer_config: &FrameBufferConfig) -> LayerManager {
        let back_buffer = FrameBuffer::new(FrameBufferConfig::new(
            screen_buffer_config.horizontal_resolution,
            screen_buffer_config.vertical_resolution,
            screen_buffer_config.pixels_per_scan_line,
            screen_buffer_config.pixel_format,
        ));

        Self {
            layers: BTreeMap::new(),
            layer_id_stack: vec![],
            latest_id: LayerID(0),
            back_buffer,
            active_layer: ActiveLayer::new(),
            layer_task_map: BTreeMap::new(),
        }
    }

    pub fn set_mouse_layer_id(&mut self, layer_id: LayerID) {
        self.active_layer.mouser_layer_id = layer_id;
    }

    pub fn get_active_layer_id(&self) -> Option<LayerID> {
        self.active_layer.active_layer_id
    }

    pub fn get_task_id_by_layer_id(&self, layer_id: LayerID) -> Option<&TaskID> {
        self.layer_task_map.get(&layer_id)
    }

    pub fn register_layer_task_relation(&mut self, layer_id: LayerID, task_id: TaskID) {
        self.layer_task_map.insert(layer_id, task_id);
    }

    pub fn new_layer(&mut self, window: Arc<Mutex<Window>>) -> &mut Layer {
        let id = self.latest_id;
        self.layers.insert(id, Layer::new(id, window));
        self.latest_id += LayerID(1); // increment after layer.push to make layer_id and index of layers equal
        self.layers.get_mut(&id).unwrap()
    }

    pub fn draw_on(&mut self, area: Rectangle<i32>) {
        for layer_id in &self.layer_id_stack {
            self.layers
                .get_mut(layer_id)
                .expect("failed to get layer")
                .draw_to(&mut self.back_buffer, area);
        }
        screen_frame_buffer()
            .lock()
            .copy(area.pos, &self.back_buffer, area);
    }

    pub fn draw_layer_of(&mut self, id: LayerID) {
        self.draw(
            id,
            Rectangle::new(Vector2D::new(0, 0), Vector2D::new(-1, -1)),
        )
    }

    fn draw(&mut self, id: LayerID, mut area: Rectangle<i32>) {
        let mut draw = false;
        let mut window_area: Rectangle<i32> = Rectangle::default();
        for &layer_id in &self.layer_id_stack {
            let layer = self.layers.get_mut(&layer_id).unwrap();

            if layer_id == id {
                window_area.size = layer.window.lock().size().to_i32_vec2d();
                window_area.pos = layer.position;
                if area.size.x >= 0 || area.size.y >= 0 {
                    area.pos += window_area.pos;
                    window_area = window_area & area;
                }
                draw = true
            }

            if draw {
                layer.draw_to(&mut self.back_buffer, window_area);
            }
        }
        screen_frame_buffer()
            .lock()
            .copy(window_area.pos, &self.back_buffer, window_area);
    }

    pub fn move_(&mut self, id: LayerID, new_position: Vector2D<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            let window_size = layer.window.lock().size();
            let old_pos = layer.position;
            layer.move_(new_position);
            self.draw_on(Rectangle::new(old_pos, window_size.to_i32_vec2d()));
            self.draw_layer_of(id);
        }
    }

    pub fn move_relative(&mut self, id: LayerID, pos_diff: Vector2D<i32>) {
        if let Some(layer) = self.layers.get_mut(&id) {
            let window_size = layer.window.lock().size();
            let old_pos = layer.position;
            layer.move_relative(pos_diff);
            self.draw_on(Rectangle::new(old_pos, window_size.to_i32_vec2d()));
            self.draw_layer_of(id);
        }
    }

    pub fn up_down(&mut self, id: LayerID, new_height: i32) {
        if self.layers.is_empty() {
            return;
        }

        if new_height.is_negative() {
            self.hide(id);
            return;
        }

        let new_height = cmp::min(new_height as usize, self.layer_id_stack.len());

        let showing_layer_id = self
            .layer_id_stack
            .iter()
            .enumerate()
            .find(|(_, &layer_id)| layer_id == id);
        match showing_layer_id {
            None => {
                // in case of the layer doesn't show yet
                assert!(self.layers.contains_key(&id)); // // check the layer exists
                self.layer_id_stack.insert(new_height, id);
            }
            Some((old_index, &layer_id)) => {
                let height = if new_height == self.layer_id_stack.len() {
                    new_height - 1 // decrement because the stack will remove
                } else {
                    new_height
                };
                self.layer_id_stack.remove(old_index);
                self.layer_id_stack.insert(height, layer_id);
            }
        }
    }

    pub fn activate_layer(&mut self, layer_id: Option<LayerID>) {
        ActiveLayer::_activate(layer_id, self);
    }

    pub fn find_layer_by_position(
        &self,
        pos: Vector2D<i32>,
        exclude_id: LayerID,
    ) -> Option<&Layer> {
        self.layer_id_stack
            .iter()
            .rev()
            .filter(|&&id| id != exclude_id)
            .map(|id| &self.layers[id])
            .find(|&layer| {
                let win_pos = layer.position;
                let win_end_pos = win_pos + layer.window.lock().size().to_i32_vec2d();
                win_pos.x <= pos.x
                    && pos.x < win_end_pos.x
                    && win_pos.y <= pos.y
                    && pos.y < win_end_pos.y
            })
    }

    pub fn process_message(&mut self, message: &LayerMessage) {
        match message.op {
            LayerOperation::Move { pos } => self.move_(message.layer_id, pos),
            LayerOperation::MoveRelative { pos } => self.move_relative(message.layer_id, pos),
            LayerOperation::Draw => self.draw_layer_of(message.layer_id),
            LayerOperation::DrawArea(area) => {
                self.draw(message.layer_id, area);
            }
        }
    }

    pub fn remove_layer(&mut self, layer_id: LayerID) {
        self.hide(layer_id);
        self.layers
            .remove(&layer_id)
            .expect("failed to remove from layers");
    }

    pub fn close_layer(&mut self, layer_id: LayerID) -> Result<(), Error> {
        let layer = match self.get_layer(layer_id) {
            None => return Err(make_error!(Code::NoSuchEntry)),
            Some(l) => l,
        };

        let pos = layer.position;
        let size = layer.get_window_ref().size();

        self.activate_layer(None);
        self.remove_layer(layer_id);
        self.draw_on(Rectangle::new(pos, size.to_i32_vec2d()));
        self.layer_task_map.remove(&layer_id);

        Ok(())
    }

    fn hide(&mut self, id: LayerID) {
        if self.layers.is_empty() {
            return;
        }

        let index = self
            .layer_id_stack
            .iter()
            .enumerate()
            .find(|(_, &layer_id)| layer_id == id);

        if let Some((index, _)) = index {
            self.layer_id_stack.remove(index);
        }
    }

    pub fn get_layer_mut(&mut self, layer_id: LayerID) -> Option<&mut Layer> {
        self.layers.get_mut(&layer_id)
    }

    pub fn get_layer(&self, layer_id: LayerID) -> Option<&Layer> {
        self.layers.get(&layer_id)
    }

    fn get_height(&self, layer_id: LayerID) -> Option<i32> {
        self.layer_id_stack
            .iter()
            .enumerate()
            .find(|(_, &id)| id == layer_id)
            .map(|(height, _)| height as i32)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LayerID(u32);

impl LayerID {
    pub const MAX: LayerID = LayerID(u32::MAX);

    pub fn new(v: u32) -> Self {
        Self(v)
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Display for LayerID {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AddAssign for LayerID {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

struct ActiveLayer {
    active_layer_id: Option<LayerID>,
    mouser_layer_id: LayerID,
}

impl ActiveLayer {
    pub const fn new() -> ActiveLayer {
        Self {
            active_layer_id: None,
            mouser_layer_id: LayerID::MAX,
        }
    }

    pub fn get_active_layer_id(&self) -> Option<LayerID> {
        self.active_layer_id
    }

    fn _activate(layer_id: Option<LayerID>, manager: &mut LayerManager) {
        if manager.active_layer.active_layer_id == layer_id {
            return;
        }

        if let Some(active_layer_id) = manager.active_layer.active_layer_id {
            let layer = manager
                .get_layer_mut(active_layer_id)
                .unwrap_or_else(|| panic!("no such layer {}", active_layer_id));
            layer.get_window_mut().deactivate();
            manager.draw_layer_of(active_layer_id);
            Self::send_window_active_message(
                active_layer_id,
                WindowActiveMode::Deactivate,
                &manager.layer_task_map,
            )
            .unwrap_or_default(); // ignore error in the same way as the official
        }

        manager.active_layer.active_layer_id = layer_id;
        if let Some(active_layer_id) = manager.active_layer.active_layer_id {
            let layer = manager
                .get_layer_mut(active_layer_id)
                .unwrap_or_else(|| panic!("no such layer {}", active_layer_id));
            layer.get_window_mut().activate();
            manager.up_down(active_layer_id, 0);
            let mouse_height = manager
                .get_height(manager.active_layer.mouser_layer_id)
                .unwrap_or(-1);
            manager.up_down(active_layer_id, mouse_height - 1);
            manager.draw_layer_of(active_layer_id);
            Self::send_window_active_message(
                active_layer_id,
                WindowActiveMode::Activate,
                &manager.layer_task_map,
            )
            .unwrap_or_default(); // ignore error in the same way as the official
        }
    }

    fn send_window_active_message(
        layer_id: LayerID,
        mode: WindowActiveMode,
        layer_task_map: &BTreeMap<LayerID, TaskID>,
    ) -> Result<(), Error> {
        if let Some(&task_id) = layer_task_map.get(&layer_id) {
            let message = Message::new(MessageType::WindowActive(mode));
            unsafe { asm!("cli") };
            let r = task_manager().send_message(task_id, message);
            unsafe { asm!("sti") };
            r
        } else {
            Err(make_error!(Code::NoSuchTask))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::global::SCREEN_FRAME_BUFFER;
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
        let id1 = lm.new_layer(Arc::new(Mutex::new(window1))).id();
        // verify layer's id equals to index of lm.layers
        assert_eq!(lm.layers[&id1].id, id1);
    }

    #[test]
    fn move_() {
        SCREEN_FRAME_BUFFER.call_once(|| init_screen_frame_buffer());
        let window1 = Window::new(1, 1, PixelFormat::KPixelBGRResv8BitPerColor);
        let mut lm = LayerManager::new(&frame_buffer_config());
        let id1 = lm.new_layer(Arc::new(Mutex::new(window1))).id();

        lm.move_(id1, Vector2D::new(100, 10));

        let l1 = &lm.layers[&id1];
        assert_eq!(l1.position, Vector2D::new(100, 10));
    }

    #[test]
    fn move_relative() {
        SCREEN_FRAME_BUFFER.call_once(|| init_screen_frame_buffer());
        let window1 = Window::new(1, 1, PixelFormat::KPixelBGRResv8BitPerColor);
        let mut lm = LayerManager::new(&frame_buffer_config());
        let mut buffer = FrameBuffer::new(frame_buffer_config());
        let id1 = lm
            .new_layer(Arc::new(Mutex::new(window1)))
            .move_(Vector2D::new(100, 100))
            .id();

        lm.move_relative(id1, Vector2D::new(-50, -30));
        {
            let l1 = &lm.layers[&id1];
            assert_eq!(l1.position, Vector2D::new(50, 70));
        }

        lm.move_relative(id1, Vector2D::new(-60, -60));
        let l1 = &lm.layers[&id1];
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
        fn window() -> Arc<Mutex<Window>> {
            Arc::new(Mutex::new(Window::new(
                1,
                1,
                PixelFormat::KPixelBGRResv8BitPerColor,
            )))
        }
        let id0 = lm.new_layer(window()).id;
        let id1 = lm.new_layer(window()).id;
        let id2 = lm.new_layer(window()).id;
        let id3 = lm.new_layer(window()).id;

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

        lm.up_down(id2, -1);
        assert_eq!(vec![id3, id0], lm.layer_id_stack);
    }

    fn init_screen_frame_buffer() -> Mutex<FrameBuffer> {
        Mutex::new(FrameBuffer::new(frame_buffer_config()))
    }
    fn frame_buffer_config() -> FrameBufferConfig {
        FrameBufferConfig::new(1, 1, 1, PixelFormat::KPixelBGRResv8BitPerColor)
    }
}
