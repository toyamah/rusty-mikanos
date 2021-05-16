#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

mod font;
mod graphics;
mod console;

use crate::graphics::{PixelColor, PixelWriter};
use core::panic::PanicInfo;
use shared::FrameBufferConfig;
use crate::console::Console;

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    let writer = PixelWriter::new(frame_buffer_config);
    write_pixel(&writer, frame_buffer_config);

    let black = PixelColor::new(0, 0, 0);
    for (i, char) in ('!'..='~').enumerate() {
        writer.write_ascii((8 * i) as u32, 50, char, &black);
    }
    writer.write_string(0, 66, "Hello, World!", &PixelColor::new(0, 0, 255));

    let white = PixelColor::new(255, 255, 255);
    let mut console = Console::new(&writer, white, black);
    console.put_string("test");

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

fn write_pixel(writer: &PixelWriter, config: &FrameBufferConfig) {
    let black = PixelColor::new(255, 255, 255);
    for x in 0..config.horizontal_resolution {
        for y in 0..config.vertical_resolution {
            writer.write(x, y, &black);
        }
    }

    let green = PixelColor::new(0, 255, 0);
    for x in 0..200 {
        for y in 0..100 {
            writer.write(x, y, &green);
        }
    }
}
