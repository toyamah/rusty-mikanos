#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

mod console;
mod font;
mod graphics;

use crate::console::Console;
use crate::graphics::{PixelColor, PixelWriter};
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

    let console = console();
    for i in 0..5 {
        console.put_string("line0\n");
        console.put_string("line1\n");
        console.put_string("line2\n");
        console.put_string("line3\n");
        console.put_string("line4\n");
    }
    console.put_string("line0\n");
    console.put_string("line1\n");

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
