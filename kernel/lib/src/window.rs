use crate::frame_buffer::FrameBuffer;
use crate::graphics::{PixelColor, PixelWriter, Rectangle, Vector2D};
use alloc::vec::Vec;
use core::cmp::{max, min};
use shared::{FrameBufferConfig, PixelFormat};

pub struct Window {
    width: usize,
    height: usize,
    //TODO: try to define without RefCel
    data: Vec<Vec<PixelColor>>,
    shadow_buffer: FrameBuffer,
    transparent_color: Option<PixelColor>,
}

impl Window {
    pub fn new(width: usize, height: usize, shadow_format: PixelFormat) -> Self {
        debug_assert!(width <= i32::MAX as usize);
        debug_assert!(height <= i32::MAX as usize);

        let data: Vec<Vec<_>> = (0..height)
            .map(|_| (0..width).map(|_| PixelColor::new(0, 0, 0)).collect())
            .collect();
        let config = FrameBufferConfig::new(width as u32, height as u32, 0, shadow_format);
        let shadow_buffer = FrameBuffer::new(config);
        Self {
            width,
            height,
            data,
            shadow_buffer,
            transparent_color: None,
        }
    }

    //TODO: change self to mutable reference if possible
    pub fn draw_to(&self, dst: &mut FrameBuffer, position: Vector2D<i32>) {
        match self.transparent_color {
            None => {
                dst.copy(position, &self.shadow_buffer);
            }
            Some(transparent) => {
                self.draw_with_transparent_to(dst, position, transparent);
            }
        }
    }

    pub fn move_(&mut self, pos: Vector2D<i32>, src: &Rectangle<i32>) {
        self.shadow_buffer.move_(pos, src)
    }

    pub fn set_transparent_color(&mut self, c: PixelColor) {
        self.transparent_color = Some(c);
    }

    pub fn writer(&mut self) -> &mut Window {
        // returns self because My Window implements PixelWriter and removed WindowPixelWriter which the Official MikanOS defined.
        self
    }

    fn draw_with_transparent_to(
        &self,
        dst: &mut FrameBuffer,
        position: Vector2D<i32>,
        transparent: PixelColor,
    ) {
        let writer = dst.writer();

        let height = self.height as i32;
        let y_start = max(0, 0 - position.y);
        let y_end = min(height, writer.height() - position.y);
        let width = self.width as i32;
        let x_start = max(0, 0 - position.x);
        let x_end = min(width, writer.width() - position.x);

        for y in y_start..y_end {
            for x in x_start..x_end {
                let color = self.at(x as usize, y as usize);
                if color != transparent {
                    dst.writer()
                        .write(position.x + x as i32, position.y + y as i32, &color);
                }
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> PixelColor {
        self.data[y][x]
    }
}

impl PixelWriter for Window {
    fn write(&mut self, x: i32, y: i32, color: &PixelColor) {
        self.data[y as usize][x as usize] = *color;
        self.shadow_buffer.writer().write(x, y, color);
    }

    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }
}
