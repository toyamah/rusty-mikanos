#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

mod console;
mod font;
mod graphics;

use crate::console::Console;
use crate::graphics::{PixelColor, PixelWriter, COLOR_BLACK, COLOR_WHITE};
use core::panic::PanicInfo;
use shared::FrameBufferConfig;

static mut PIXEL_WRITER: Option<PixelWriter> = None;

fn pixel_writer() -> &'static mut PixelWriter<'static> {
    unsafe { PIXEL_WRITER.as_mut().unwrap() }
}

static mut CONSOLE: Option<Console> = None;

fn console() -> &'static mut Console<'static> {
    unsafe { CONSOLE.as_mut().unwrap() }
}

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_config: &'static FrameBufferConfig) -> ! {
    initialize_global_vars(frame_buffer_config);

    write_pixel(pixel_writer(), frame_buffer_config);
    write_cursor();

    // for i in 0..27 {
    //     printk!("line {}\n", i);
    // }

    loop {
        unsafe { asm!("hlt") }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

fn initialize_global_vars(frame_buffer_config: &'static FrameBufferConfig) {
    unsafe {
        PIXEL_WRITER = Some(PixelWriter::new(frame_buffer_config));
    }

    let white = PixelColor::new(255, 255, 255);
    let black = PixelColor::new(0, 0, 0);
    unsafe {
        CONSOLE = Some(Console::new(pixel_writer(), white, black));
    }
}

fn write_pixel(writer: &PixelWriter, config: &FrameBufferConfig) {
    let black = PixelColor::new(0, 0, 0);
    for x in 0..config.horizontal_resolution {
        for y in 0..config.vertical_resolution {
            writer.write(x, y, &black);
        }
    }
}

fn write_cursor() {
    let writer = pixel_writer();
    for (dy, str) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in str.chars().enumerate() {
            if char == '@' {
                writer.write((200 + dx) as u32, (100 + dy) as u32, &COLOR_WHITE);
            } else if char == '.' {
                writer.write((200 + dx) as u32, (100 + dy) as u32, &COLOR_BLACK);
            };
        }
    }
}

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
