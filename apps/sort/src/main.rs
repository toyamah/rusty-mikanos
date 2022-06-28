#![no_std]
#![no_main]
#![feature(format_args_nl)]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_string, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let path = if args.len() >= 2 {
        args.get(1)
    } else {
        "@stdin"
    };

    let fp = open_file(path, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 1024];
    let mut lines = vec![];
    while !read_string(fp, &mut line).is_null() {
        lines.push(buf_to_str(&line).unwrap().to_string());
    }

    lines.sort();

    for line in lines {
        print!("{}", line);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
