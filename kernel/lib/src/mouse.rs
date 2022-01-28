use crate::graphics::{PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE};
use crate::Window;
use shared::PixelFormat;

const MOUSE_TRANSPARENT_COLOR: PixelColor = PixelColor::new(0, 0, 1);
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

pub fn draw_mouse_cursor<W: PixelWriter>(pixel_writer: &W, position: &Vector2D<i32>) {
    // pub fn draw_mouse_cursor(pixel_writer: &FrameBufferWriter, position: &Vector2D<i32>) {
    for (dy, row) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in row.chars().enumerate() {
            let color = match char {
                '@' => &COLOR_WHITE,
                '.' => &COLOR_BLACK,
                _ => &MOUSE_TRANSPARENT_COLOR,
            };
            pixel_writer.write(position.x + dx as i32, position.y + dy as i32, color);
        }
    }
}

pub fn new_mouse_cursor_window(pixel_format: PixelFormat) -> Window {
    let mut window = Window::new(
        MOUSE_CURSOR_SHAPE[0].len(),
        MOUSE_CURSOR_SHAPE.len(),
        pixel_format,
    );
    window.set_transparent_color(MOUSE_TRANSPARENT_COLOR);
    window
}
