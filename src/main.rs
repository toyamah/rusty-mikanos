#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain() -> ! {
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
