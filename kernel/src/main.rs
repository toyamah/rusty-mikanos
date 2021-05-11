#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_base: u64, frame_buffer_size: u64) -> ! {
    let frame_buffer = frame_buffer_base as *mut u8;
    for i in 0..frame_buffer_size {
        unsafe {
            *frame_buffer.offset(i as isize) = (i % 256) as u8;
        }
    }

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
