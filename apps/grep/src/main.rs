#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use safe_regex::{regex, Matcher0};
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_file, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    if args.len() < 3 {
        println!("Usage: {} <pattern> <file>\n", args.get(0));
        exit(1);
    }
    let path = args.get(2);

    // Because implementing regexp is not the essence of OS, supported patterns are:
    let matcher: Matcher0<_> = match args.get(1) {
        "3FE.D000" => regex!(br".*3FE.D000.*"),
        // TODO: Add another pattern in day26
        p => {
            println!("Unsupported pattern: {}", p);
            exit(1);
        }
    };

    let fp = open_file(path, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 256];
    while !read_file(fp, line.as_mut()).is_null() {
        if matcher.is_match(line.as_slice()) {
            let str = buf_to_str(&line).unwrap();
            print!("{}", str);
        }
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}