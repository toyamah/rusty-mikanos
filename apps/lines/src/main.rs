#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::libc::atol;
use shared_lib::newlib_support::exit;
use shared_lib::println;
use shared_lib::rust_official::cchar::c_char;

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    println!("lines");
    exit(1);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}