#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;

const RADIUS: i32 = 90;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    exit(0)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
