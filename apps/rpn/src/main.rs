#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::rust_official::cstr::CStr;

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) -> i32 {
    let mut sum: i32 = 0;
    for i in 1..argc {
        let ptr = unsafe { *argv.offset(i as isize) };
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let bytes = c_str.to_bytes();

        let mut v = 0;
        for &x in bytes {
            v = v * 10 + i32::from(x - b'0');
        }
        sum += v;
    }

    sum
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
