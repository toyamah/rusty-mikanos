use crate::graphics::PixelWriter;
use crate::window::Window;
use crate::{FrameBufferWriter, Vector2D};
use alloc::vec;
use alloc::vec::Vec;

pub struct Layer<'a> {
    id: u32,
    position: Vector2D<usize>,
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

    pub fn move_(&mut self, pos: Vector2D<usize>) -> &mut Layer<'a> {
        self.position = pos;
        self
    }

    pub fn move_relative(&mut self, diff: Vector2D<i32>) {
        let x = usize_add(self.position.x, diff.x);
        let y = usize_add(self.position.y, diff.y);
        self.position = Vector2D::new(x, y)
    }

    pub fn draw_to(&self, writer: &mut FrameBufferWriter) {
        if let Some(w) = self.window {
            w.draw_to(writer, self.position)
        }
    }
}

fn usize_add(u: usize, i: i32) -> usize {
    if i.is_negative() {
        u.checked_sub(i.wrapping_abs() as u32 as usize)
    } else {
        u.checked_sub(i as usize)
    }
    .unwrap()
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
        self.layers.first_mut().unwrap()
    }
}
