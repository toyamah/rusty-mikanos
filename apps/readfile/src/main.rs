#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_file, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let path = if args.len() < 2 {
        "/memmap"
    } else {
        args.get(1)
    };

    let fp = open_file(path, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 256];
    for _ in 0..3 {
        if read_file(fp, &mut line).is_null() {
            println!("failed to get a line");
        }
        let str = buf_to_str(&line).unwrap();
        print!("{}", str);
    }
    println!("----");

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
