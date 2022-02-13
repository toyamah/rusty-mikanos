use crate::frame_buffer::FrameBuffer;
use crate::graphics::{
    fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_BLACK, COLOR_WHITE,
};
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

    pub fn size(&self) -> Vector2D<usize> {
        Vector2D::new(self.width, self.height)
    }

    //TODO: change self to mutable reference if possible
    pub fn draw_to(&self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: Rectangle<i32>) {
        match self.transparent_color {
            None => {
                let window_area = Rectangle::new(pos, self.size().to_i32_vec2d());
                let intersection = area & window_area;
                dst.copy(
                    intersection.pos,
                    &self.shadow_buffer,
                    Rectangle::new(intersection.pos - pos, intersection.size),
                );
            }
            Some(transparent) => {
                self.draw_with_transparent_to(dst, pos, transparent);
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

    pub fn draw_window(&mut self, title: &str) {
        let win_w = self.width as i32;
        let win_h = self.height as i32;

        self.fill_rect((0, 0), (win_w, 1), 0xc6c6c6);
        self.fill_rect((1, 1), (win_w - 2, 1), 0xffffff);
        self.fill_rect((0, 0), (1, win_h), 0xc6c6c6);
        self.fill_rect((1, 1), (1, win_h - 2), 0xffffff);
        self.fill_rect((win_w - 2, 1), (1, win_h - 2), 0x848484);
        self.fill_rect((win_w - 1, 0), (1, win_h), 0x000000);
        self.fill_rect((2, 2), (win_w - 4, win_h - 4), 0xc6c6c6);
        self.fill_rect((3, 3), (win_w - 6, 18), 0x000084);
        self.fill_rect((1, win_h - 2), (win_w - 2, 1), 0x848484);
        self.fill_rect((0, win_h - 1), (win_w, 1), 0x000000);

        self.write_string(24, 4, title, &PixelColor::from(0xffffff));

        for (y, &str) in CLOSE_BUTTON.iter().enumerate() {
            for (x, char) in str.chars().enumerate() {
                let color = match char {
                    '@' => COLOR_WHITE,
                    '$' => PixelColor::from(0x848484),
                    ':' => PixelColor::from(0xc6c6c6),
                    _ => COLOR_BLACK,
                };
                self.write(
                    win_w - 5 - str.len() as i32 + x as i32,
                    (5 + y) as i32,
                    &color,
                );
            }
        }
    }

    fn draw_with_transparent_to(
        &self,
        dst: &mut FrameBuffer,
        pos: Vector2D<i32>,
        transparent: PixelColor,
    ) {
        let writer = dst.writer();

        let height = self.height as i32;
        let y_start = max(0, 0 - pos.y);
        let y_end = min(height, writer.height() - pos.y);
        let width = self.width as i32;
        let x_start = max(0, 0 - pos.x);
        let x_end = min(width, writer.width() - pos.x);

        for y in y_start..y_end {
            for x in x_start..x_end {
                let color = self.at(x as usize, y as usize);
                if color != transparent {
                    dst.writer()
                        .write(pos.x + x as i32, pos.y + y as i32, &color);
                }
            }
        }
    }

    fn at(&self, x: usize, y: usize) -> PixelColor {
        self.data[y][x]
    }

    fn fill_rect(&mut self, pos: (i32, i32), size: (i32, i32), c: u32) {
        fill_rectangle(
            self,
            &Vector2D::new(pos.0, pos.1),
            &Vector2D::new(size.0, size.1),
            &PixelColor::from(c),
        )
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

const CLOSE_BUTTON: [&str; 14] = [
    "...............@",
    ".:::::::::::::$@",
    ".:::::::::::::$@",
    ".:::@@::::@@::$@",
    ".::::@@::@@:::$@",
    ".:::::@@@@::::$@",
    ".::::::@@:::::$@",
    ".:::::@@@@::::$@",
    ".::::@@::@@:::$@",
    ".:::@@::::@@::$@",
    ".:::::::::::::$@",
    ".:::::::::::::$@",
    ".$$$$$$$$$$$$$$@",
    "@@@@@@@@@@@@@@@@",
];
