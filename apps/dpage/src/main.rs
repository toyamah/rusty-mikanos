#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::ffi::c_void;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::file::{open_file, read_file_raw, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{demand_page, println};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let (filename, ch) = if args.len() < 3 {
        ("/memmap", b'\n')
    } else {
        (args.get(1), args.get_byte(2))
    };

    let fp = open_file(filename, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", filename);
        exit(1);
    }

    let mut buf = match demand_page(1, 0) {
        Ok(v) => v as *mut u8,
        Err(e) => exit(1),
    };
    let buf0 = buf;

    let mut total = 0;
    let mut n = 0;
    loop {
        n = read_file_raw(fp, buf as *mut c_void, 1, 4096);
        if n != 4096 {
            break;
        }
        total += n;
        if demand_page(1, 0).is_err() {
            exit(1);
        }
        unsafe {
            buf = buf.add(4096);
        }
    }
    total += n;
    println!("size of {} = {} bytes", filename, total);

    let mut num = 0;
    for i in 0..total {
        let b = unsafe { *buf0.add(i) };
        if b == ch {
            num += 1;
        }
    }

    println!("the number of {} = {}", ch, num);

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
