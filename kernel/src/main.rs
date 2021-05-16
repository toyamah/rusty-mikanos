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

    let white = PixelColor::new(255, 255, 255);
    let black = PixelColor::new(0, 0, 0);
    let mut console = Console::new(&writer, white, black);

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

fn write_pixel(writer: &PixelWriter, config: &FrameBufferConfig) {
    let black = PixelColor::new(0, 0, 0);
    for x in 0..config.horizontal_resolution {
        for y in 0..config.vertical_resolution {
            writer.write(x, y, &black);
        }
    }
}
