use crate::graphics::{
    FrameBufferWriter, PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE,
};
use crate::Window;

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

pub struct MouseCursor<'a> {
    writer: &'a FrameBufferWriter<'a>,
    erace_color: &'a PixelColor,
    position: Vector2D<i32>,
}

impl<'a> MouseCursor<'a> {
    pub fn new(
        writer: &'a FrameBufferWriter<'a>,
        erace_color: &'a PixelColor,
        initial_position: Vector2D<i32>,
    ) -> Self {
        Self {
            writer,
            erace_color,
            position: initial_position,
        }
    }

    pub fn draw(&self) {
        erase_mouse_cursor(self.writer, &self.position, self.erace_color);
        draw_mouse_cursor(self.writer, &self.position);
    }

    pub fn move_relative(&mut self, displacement: &Vector2D<i32>) {
        erase_mouse_cursor(self.writer, &self.position, self.erace_color);
        self.position += *displacement;
        draw_mouse_cursor(self.writer, &self.position);
    }
}

fn erase_mouse_cursor<'a>(
    pixel_writer: &'a FrameBufferWriter,
    position: &Vector2D<i32>,
    erase_color: &PixelColor,
) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in row.chars().enumerate() {
            if c != ' ' {
                let x = position.x + dx as i32;
                let y = position.y + dy as i32;
                pixel_writer.write(x, y, erase_color);
            }
        }
    }
}

pub fn draw_mouse_cursor<W: PixelWriter>(pixel_writer: &W, position: &Vector2D<i32>) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in row.chars().enumerate() {
            if char == '@' {
                pixel_writer.write(position.x + dx as i32, position.y + dy as i32, &COLOR_WHITE);
            } else if char == '.' {
                pixel_writer.write(position.x + dx as i32, position.y + dy as i32, &COLOR_BLACK);
            }
        }
    }
}

pub fn new_mouse_cursor_window() -> Window {
    Window::new(MOUSE_CURSOR_SHAPE[0].len(), MOUSE_CURSOR_SHAPE.len())
}
