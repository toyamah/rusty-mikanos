#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

mod pixel_writer;

use crate::pixel_writer::{
    write_pixel, BGRResv8BitPerColorPixelWriter, RGBResv8BitPerColorPixelWriter,
};
use core::panic::PanicInfo;
use shared::{FrameBufferConfig, PixelFormat};

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_config: &FrameBufferConfig) -> ! {
    match frame_buffer_config.pixel_format {
        PixelFormat::KPixelRGBResv8BitPerColor => {
            let writer = RGBResv8BitPerColorPixelWriter::new(frame_buffer_config);
            write_pixel(&writer, frame_buffer_config);
        }
        PixelFormat::KPixelBGRResv8BitPerColor => {
            let writer = BGRResv8BitPerColorPixelWriter::new(frame_buffer_config);
            write_pixel(&writer, frame_buffer_config);
        }
    };

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
