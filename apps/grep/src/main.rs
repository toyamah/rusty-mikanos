#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use safe_regex::{regex, Matcher0};
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_string, OpenMode};
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
    match args.get(1) {
        "3FE.D000" => grep(path, regex!(br".*3FE.D000.*")),
        "i.t" => grep(path, regex!(br".*i.t.*")),
        "Conv" => grep(path, regex!(br".*Conv.*")),
        "3E" => grep(path, regex!(br".*3E.*")),
        "N" => grep(path, regex!(br".*N.*")),
        p => {
            println!("Unsupported pattern: {}", p);
            exit(1);
        }
    };

    exit(0);
}

fn grep<F: Fn(&[u8]) -> Option<()>>(path: &str, matcher: Matcher0<F>) {
    let fp = open_file(path, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 256];
    while !read_string(fp, line.as_mut()).is_null() {
        if matcher.is_match(line.as_slice()) {
            let str = buf_to_str(&line).unwrap();
            print!("{}", str);
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
