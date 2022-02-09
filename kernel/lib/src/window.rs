use crate::frame_buffer::FrameBuffer;
use crate::graphics::{PixelColor, PixelWriter, Rectangle, Vector2D};
use alloc::vec::Vec;
use core::cell::RefCell;
use shared::{FrameBufferConfig, PixelFormat};

pub struct Window {
    width: usize,
    height: usize,
    //TODO: try to define without RefCel
    data: RefCell<Vec<Vec<PixelColor>>>,
    shadow_buffer: RefCell<FrameBuffer>,
    transparent_color: Option<PixelColor>,
}

impl Window {
    pub fn new(width: usize, height: usize, shadow_format: PixelFormat) -> Self {
        debug_assert!(width <= i32::MAX as usize);
        debug_assert!(height <= i32::MAX as usize);

        let data: Vec<Vec<_>> = (0..height)
            .map(|_| (0..width).map(|_| PixelColor::new(0, 0, 0)).collect())
            .collect();
        let data = RefCell::new(data);
        let config = FrameBufferConfig::new(width as u32, height as u32, 0, shadow_format);
        let shadow_buffer = RefCell::new(FrameBuffer::new(config));
        Self {
            width,
            height,
            data,
            shadow_buffer,
            transparent_color: None,
        }
    }

    pub fn draw_to(&self, dst: &mut FrameBuffer, position: Vector2D<i32>) {
        match self.transparent_color {
            None => {
                dst.copy(position, &self.shadow_buffer.borrow());
            }
            Some(transparent) => self.on_each_pixel(move |x, y| {
                let color = self.at(x, y);
                if color != transparent {
                    dst.writer()
                        .write(position.x + x as i32, position.y + y as i32, &color);
                }
            }),
        }
    }

    pub fn move_(&mut self, pos: Vector2D<i32>, src: &Rectangle<i32>) {
        self.shadow_buffer.borrow_mut().move_(pos, src)
    }

    pub fn set_transparent_color(&mut self, c: PixelColor) {
        self.transparent_color = Some(c);
    }

    pub fn writer(&mut self) -> &Window {
        // returns self because My Window implements PixelWriter and removed WindowPixelWriter which the Official MikanOS defined.
        self
    }

    fn on_each_pixel<F>(&self, mut f: F)
    where
        F: FnMut(usize, usize) -> (),
    {
        for y in 0..self.height {
            for x in 0..self.width {
                f(x, y);
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> PixelColor {
        self.data.borrow()[y][x]
    }
}

impl PixelWriter for Window {
    fn write(&self, x: i32, y: i32, color: &PixelColor) {
        self.data.borrow_mut()[y as usize][x as usize] = *color;
        self.shadow_buffer.borrow_mut().writer().write(x, y, color);
    }

    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }
}
