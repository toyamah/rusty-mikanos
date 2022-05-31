#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::file::{map_file, open_file, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println};

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let fd = open_file("/memmap", OpenMode::R);

    let mut file_size = 0;
    let p = match map_file(fd, &mut file_size, 0) {
        Ok(p) => p as *const u64 as *const u8,
        Err(e) => exit(e.error_number()),
    };

    for i in 0..file_size {
        let b = unsafe { *p.add(i) };
        print!("{}", char::from(b));
    }
    println!("\nread from mapped file {} bytes", file_size);

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
