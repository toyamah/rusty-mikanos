use crate::graphics::{PixelWriter, Vector2D};
use crate::window::Window;
use alloc::vec;
use alloc::vec::Vec;

pub struct Layer<'a> {
    id: u32,
    position: Vector2D<i32>,
    window: Option<&'a Window>,
}

impl<'a> Layer<'a> {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            position: Vector2D::new(0, 0),
            window: None,
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

    pub fn move_(&mut self, pos: Vector2D<i32>) -> &mut Layer<'a> {
        self.position = pos;
        self
    }

    pub fn move_relative(&mut self, diff: Vector2D<i32>) {
        let x = self.position.x + diff.x;
        let y = self.position.y + diff.y;
        self.position = Vector2D::new(x, y)
    }

    pub fn draw_to<W: PixelWriter + ?Sized>(&self, writer: &W) {
        if let Some(w) = self.window {
            w.draw_to(writer, self.position)
        }
    }
}

pub struct LayerManager<'a> {
    writer: &'a dyn PixelWriter,
    layers: Vec<Layer<'a>>,
    layer_stack: Vec<&'a Layer<'a>>,
    latest_id: u32,
}

impl<'a> LayerManager<'a> {
    pub fn new<W: PixelWriter>(writer: &'a W) -> LayerManager<'a> {
        Self {
            writer,
            layers: vec![],
            layer_stack: vec![],
            latest_id: 0,
        }
    }

    pub fn set_writer<W: PixelWriter>(&mut self, writer: &'a W) {
        self.writer = writer
    }

    pub fn new_layer(&mut self) -> &mut Layer<'a> {
        self.latest_id += 1;
        self.layers.push(Layer::new(self.latest_id));
        self.layers.iter_mut().last().unwrap()
    }

    pub fn draw(&mut self) {
        for &l in &self.layer_stack {
            l.draw_to(self.writer)
        }
    }

    pub fn move_(&mut self, id: u32, new_position: Vector2D<i32>) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.move_(new_position);
        }
    }

    pub fn move_relative(&mut self, id: u32, pos_diff: Vector2D<i32>) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.move_relative(pos_diff);
        }
    }

    pub fn up_down(&'a mut self, id: u32, new_height: i32) {
        if self.layers.is_empty() {
            return;
        }

        if new_height.is_negative() {
            self.hide(id);
            return;
        }

        let new_height = {
            let h = new_height as usize;
            if h > self.layer_stack.len() {
                self.layer_stack.len()
            } else {
                h
            }
        };

        match self
            .layer_stack
            .iter()
            .enumerate()
            .find(|(_, &l)| l.id == id)
        {
            None => {
                // in case of the layer doesn't show yet
                let layer = self.layers.iter().find(|l| l.id == id).unwrap();
                self.layer_stack.push(layer);
            }
            Some((old_index, &layer)) => {
                let height = if new_height == self.layer_stack.len() - 1 {
                    new_height - 1 // decrement because the stack will remove
                } else {
                    new_height
                };
                self.layer_stack.remove(old_index);
                self.layer_stack.insert(height - 1, layer);
            }
        }
    }

    fn hide(&mut self, id: u32) {
        if self.layers.is_empty() {
            return;
        }

        let last_id = self.layer_stack.last().unwrap().id;
        let hiding_index = self
            .layers
            .iter()
            .position(|l| l.id == id && l.id != last_id);

        if let Some(i) = hiding_index {
            self.layer_stack.remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::PixelColor;
    use core::cell::RefCell;

    struct MockWriter {
        width: i32,
        height: i32,
        write_history: RefCell<Vec<(i32, i32, PixelColor)>>,
    }

    impl MockWriter {
        fn new(width: i32, height: i32) -> Self {
            Self {
                width,
                height,
                write_history: RefCell::new(Vec::new()),
            }
        }
    }

    impl PixelWriter for MockWriter {
        fn write(&self, x: i32, y: i32, color: &PixelColor) {
            self.write_history.borrow_mut().push((x, y, *color))
        }

        fn width(&self) -> i32 {
            self.width
        }

        fn height(&self) -> i32 {
            self.height
        }
    }

    #[test]
    fn move_() {
        let writer = MockWriter::new(1, 2);
        let window1 = Window::new(1, 1);
        let mut lm = LayerManager::new(&writer);
        let id1 = lm.new_layer().set_window(&window1).id();

        lm.move_(id1, Vector2D::new(100, 10));

        assert_eq!(layer(&lm, id1).position, Vector2D::new(100, 10));
    }

    #[test]
    fn move_relative() {
        let writer = MockWriter::new(1, 2);
        let window1 = Window::new(1, 1);
        let mut lm = LayerManager::new(&writer);
        let id1 = lm
            .new_layer()
            .set_window(&window1)
            .move_(Vector2D::new(100, 100))
            .id();

        lm.move_relative(id1, Vector2D::new(-50, -30));
        assert_eq!(layer(&lm, id1).position, Vector2D::new(50, 70));

        lm.move_relative(id1, Vector2D::new(-60, -60));
        assert_eq!(layer(&lm, id1).position, Vector2D::new(-10, 10));
    }

    fn layer<'a>(lm: &'a LayerManager, id: u32) -> &'a Layer<'a> {
        lm.layers.iter().find(|l| l.id == id).unwrap()
    }
}
