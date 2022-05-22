#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::libc::{fgets, fopen};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::rust_official::cstr::CStr;
use shared_lib::{print, println};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let path = if args.len() < 2 {
        "/memmap\0"
    } else {
        args.get(1)
    };

    let p = path.as_ptr() as *const c_char;
    let fp = unsafe { fopen(p, "r".as_ptr() as *const c_char) };
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 256];
    let buf = &mut line as *mut u8 as *mut c_char;
    for _ in 0..3 {
        if unsafe { fgets(buf, line.len() as i32, fp) }.is_null() {
            println!("failed to get a line");
        }
        let str = unsafe { CStr::from_ptr(buf as *const c_char) }
            .to_str()
            .unwrap();
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
