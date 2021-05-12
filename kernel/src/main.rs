#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use shared::{FrameBufferConfig, PixelFormat};

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    let black = PixelColor {
        r: 255,
        g: 255,
        b: 255,
    };
    for x in 0..frame_buffer_config.horizontal_resolution {
        for y in 0..frame_buffer_config.vertical_resolution {
            write_pixel(frame_buffer_config, x, y, &black);
        }
    }
    let green= PixelColor {
        r: 0,
        g: 255,
        b: 0,
    };
    for x in 0..200 {
        for y in 0..100 {
            write_pixel(frame_buffer_config, 100 + x, 100 + y, &green);
        }
    }

    loop {
        unsafe { asm!("hlt") }
    }
}

struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

fn write_pixel(config: &FrameBufferConfig, x: u32, y: u32, c: &PixelColor) {
    let pixel_position = config.pixels_per_scan_line * y + x;
    let base: isize = (4 * pixel_position) as isize;
    if config.pixel_format == PixelFormat::KPixelRGBResv8BitPerColor {
        unsafe {
            let p = config.frame_buffer.offset(base);
            *p.offset(0) = c.r;
            *p.offset(1) = c.g;
            *p.offset(2) = c.b;
        }
    } else {
        unsafe {
            let p = config.frame_buffer.offset(base);
            *p.offset(0) = c.b;
            *p.offset(1) = c.g;
            *p.offset(2) = c.r;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
