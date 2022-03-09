use crate::frame_buffer::FrameBuffer;
use crate::graphics::{
    fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_BLACK, COLOR_WHITE,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::{max, min};
use shared::{FrameBufferConfig, PixelFormat};

const TOP_LEFT_MARGIN: Vector2D<i32> = Vector2D::new(4, 24);
const BOTTOM_RIGHT_MARGIN: Vector2D<i32> = Vector2D::new(4, 4);

pub enum Type {
    Normal,
    TopLevel { title: String },
}

pub struct Window {
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    shadow_buffer: FrameBuffer,
    transparent_color: Option<PixelColor>,
    type_: Type,
}

impl Window {
    pub fn new(width: usize, height: usize, shadow_format: PixelFormat) -> Self {
        Window::_new(width, height, shadow_format, Type::Normal)
    }

    pub fn new_with_title(
        width: usize,
        height: usize,
        shadow_format: PixelFormat,
        title: &str,
    ) -> Window {
        let mut w = Window::_new(
            width,
            height,
            shadow_format,
            Type::TopLevel {
                title: title.to_string(),
            },
        );
        draw_window(&mut w.normal_window_writer(), title);
        w
    }

    fn _new(width: usize, height: usize, shadow_format: PixelFormat, type_: Type) -> Window {
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
            type_,
        }
    }

    pub fn size(&self) -> Vector2D<usize> {
        Vector2D::new(self.width, self.height)
    }

    pub fn draw_to(&mut self, dst: &mut FrameBuffer, pos: Vector2D<i32>, area: Rectangle<i32>) {
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

    pub fn activate(&mut self) {
        match &self.type_ {
            Type::Normal => {}
            Type::TopLevel { title } => {
                let title = title.to_string();
                draw_window_title(&mut self.normal_window_writer(), title.as_str(), true);
            }
        }
    }

    pub fn deactivate(&mut self) {
        match &self.type_ {
            Type::Normal => {}
            Type::TopLevel { title } => {
                let title = title.to_string();
                draw_window_title(&mut self.normal_window_writer(), title.as_str(), false);
            }
        }
    }

    pub fn inner_size(&self) -> Vector2D<i32> {
        match self.type_ {
            Type::Normal => Vector2D::new(0, 0),
            Type::TopLevel { .. } => {
                self.size().to_i32_vec2d() - TOP_LEFT_MARGIN - BOTTOM_RIGHT_MARGIN
            }
        }
    }

    pub fn draw_text_box(&mut self, pos: Vector2D<i32>, size: Vector2D<i32>) {
        // fill main box
        fill_rect(
            self,
            (pos.x + 1, pos.y + 1),
            (size.x - 2, size.y - 2),
            0xffffff,
        );

        // draw border lines
        fill_rect(self, (pos.x, pos.y), (size.x, 1), 0x848484);
        fill_rect(self, (pos.x, pos.y), (1, size.y), 0x848484);
        fill_rect(self, (pos.x, pos.y + size.y), (size.x, 1), 0xc6c6c6);
        fill_rect(self, (pos.x + size.x, pos.y), (1, size.y), 0xc6c6c6);
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

    /// Official TopLevelWindow sometimes uses Writer() which is a WindowWriter not an InnerAreaWriter.
    /// This method can be used where the official code uses TopLevelWindow.Writer().
    fn normal_window_writer(&mut self) -> WindowWriter {
        WindowWriter { w: self }
    }
}

fn draw_window<W: PixelWriter>(writer: &mut W, title: &str) {
    let win_w = writer.width() as i32;
    let win_h = writer.height() as i32;

    fill_rect(writer, (0, 0), (win_w, 1), 0xc6c6c6);
    fill_rect(writer, (1, 1), (win_w - 2, 1), 0xffffff);
    fill_rect(writer, (0, 0), (1, win_h), 0xc6c6c6);
    fill_rect(writer, (1, 1), (1, win_h - 2), 0xffffff);
    fill_rect(writer, (win_w - 2, 1), (1, win_h - 2), 0x848484);
    fill_rect(writer, (win_w - 1, 0), (1, win_h), 0x000000);
    fill_rect(writer, (2, 2), (win_w - 4, win_h - 4), 0xc6c6c6);
    fill_rect(writer, (3, 3), (win_w - 6, 18), 0x000084);
    fill_rect(writer, (1, win_h - 2), (win_w - 2, 1), 0x848484);
    fill_rect(writer, (0, win_h - 1), (win_w, 1), 0x000000);

    draw_window_title(writer, title, false);
}

fn fill_rect<W: PixelWriter>(writer: &mut W, pos: (i32, i32), size: (i32, i32), c: u32) {
    fill_rectangle(
        writer,
        &Vector2D::new(pos.0, pos.1),
        &Vector2D::new(size.0, size.1),
        &PixelColor::from(c),
    )
}

const COLOR_848484: PixelColor = PixelColor::new(0x84, 0x84, 0x84);
const COLOR_C6C6C6: PixelColor = PixelColor::new(0xc6, 0xc6, 0xc6);
const COLOR_000084: PixelColor = PixelColor::new(0x00, 0x00, 0x84);

fn draw_window_title<W: PixelWriter>(writer: &mut W, title: &str, is_active: bool) {
    let win_w = writer.width() as i32;
    let bg_color = if is_active {
        &COLOR_000084
    } else {
        &COLOR_848484
    };

    fill_rectangle(
        writer,
        &Vector2D::new(3, 3),
        &Vector2D::new(win_w - 6, 18),
        bg_color,
    );
    writer.write_string(24, 4, title, &PixelColor::from(0xffffff));

    for (y, &str) in CLOSE_BUTTON.iter().enumerate() {
        for (x, char) in str.chars().enumerate() {
            let color = match char {
                '@' => &COLOR_WHITE,
                '$' => &COLOR_848484,
                ':' => &COLOR_C6C6C6,
                _ => &COLOR_BLACK,
            };
            writer.write(
                win_w - 5 - str.len() as i32 + x as i32,
                (5 + y) as i32,
                color,
            );
        }
    }
}

fn write_w(w: &mut Window, x: i32, y: i32, color: &PixelColor) {
    w.data[y as usize][x as usize] = *color;
    w.shadow_buffer.writer().write(x, y, color);
}

impl PixelWriter for Window {
    fn write(&mut self, x: i32, y: i32, color: &PixelColor) {
        match self.type_ {
            Type::Normal => write_w(self, x, y, color),
            Type::TopLevel { .. } => {
                write_w(self, x + TOP_LEFT_MARGIN.x, y + TOP_LEFT_MARGIN.y, color)
            }
        }
    }

    fn width(&self) -> i32 {
        match self.type_ {
            Type::Normal => self.width as i32,
            Type::TopLevel { .. } => self.width as i32 - TOP_LEFT_MARGIN.x - BOTTOM_RIGHT_MARGIN.x,
        }
    }

    fn height(&self) -> i32 {
        match self.type_ {
            Type::Normal => self.height as i32,
            Type::TopLevel { .. } => self.height as i32 - TOP_LEFT_MARGIN.x - BOTTOM_RIGHT_MARGIN.x,
        }
    }
}

struct WindowWriter<'a> {
    w: &'a mut Window,
}

impl<'a> PixelWriter for WindowWriter<'a> {
    fn write(&mut self, x: i32, y: i32, color: &PixelColor) {
        write_w(self.w, x, y, color)
    }

    fn width(&self) -> i32 {
        self.w.width as i32
    }

    fn height(&self) -> i32 {
        self.w.height as i32
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
