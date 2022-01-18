use crate::graphics::PixelWriter;
use crate::{FrameBufferWriter, PixelColor, Vector2D};
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use core::borrow::BorrowMut;

pub struct Window<'a> {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    transparent_color: Option<&'a PixelColor>,
    writer: WindowWriter,
}

pub struct WindowWriter;

impl PixelWriter for WindowWriter {
    fn write(&self, x: i32, y: i32, color: &PixelColor) {
        todo!()
    }

    fn width(&self) -> i32 {
        todo!()
    }

    fn height(&self) -> i32 {
        todo!()
    }
}

impl<'a> Window<'a> {
    pub fn new(width: usize, height: usize) -> Window<'a> {
        let mut data = (0..height).map(|_| Vec::with_capacity(width)).collect();
        Self {
            width,
            height,
            data,
            transparent_color: None,
            writer: WindowWriter,
        }
    }

    pub fn draw_to(&self, writer: &'a FrameBufferWriter, position: Vector2D<usize>) {
        match self.transparent_color {
            None => self.on_each_pixel(|x, y| {
                writer.write(
                    (position.x + x) as i32,
                    (position.y + y) as i32,
                    &self.at(x, y),
                )
            }),
            Some(transparent) => self.on_each_pixel(|x, y| {
                let color = self.at(x, y);
                if color != transparent {
                    writer.write((position.x + x) as i32, (position.y + y) as i32, color);
                }
            }),
        }
    }

    pub fn set_transparent_color(&mut self, c: Option<&'a PixelColor>) {
        self.transparent_color = c
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    // pub fn writer(&self) -> &'a WindowWriter {
    //     &self.writer
    // }

    fn on_each_pixel<F>(&self, f: F)
    where
        F: Fn(usize, usize) -> (),
    {
        for y in 0..self.height {
            for x in 0..self.width {
                f(x, y);
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> &PixelColor {
        &self.data[y][x]
    }
}
