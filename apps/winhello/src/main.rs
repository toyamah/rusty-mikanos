#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{open_window, write_string_to_window};

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let layer_id = match open_window(200, 100, 10, 10, "winhello") {
        Ok(id) => id,
        Err(e) => exit(e.error_number()),
    };

    write_string_to_window(layer_id, 7, 24, 0xc00000, "hello world!");
    write_string_to_window(layer_id, 24, 40, 0x00c000, "hello world!");
    write_string_to_window(layer_id, 40, 56, 0x0000c0, "hello world!");

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
