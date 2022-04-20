#![no_std]
#![no_main]
#![allow(dead_code)]
use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::atol;
use shared_lib::rust_official::cchar::c_char;

const TABLE: [u8; 3 * 1024 * 1024] = [0; 3 * 1024 * 1024];

#[no_mangle]
pub extern "C" fn main(_argc: i32, argv: *const *const c_char) -> i32 {
    let arg1 = unsafe { *argv.add(1 as usize) };
    (unsafe { atol(arg1) }) as i32
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
