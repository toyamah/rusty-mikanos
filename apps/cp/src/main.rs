#![no_std]
#![no_main]
#![feature(format_args_nl)]
use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::file::{open_file, read_file2, write_file, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::println;
use shared_lib::rust_official::cchar::c_char;

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    if args.len() < 3 {
        println!("Usage: {} <src> <dest>", args.get(0));
        exit(1);
    }

    let fp_src = open_file(args.get(1), OpenMode::R);
    if fp_src.is_null() {
        println!("failed to open for read: {}", args.get(1));
        exit(1);
    }

    let fp_dest = open_file(args.get(2), OpenMode::W);
    if fp_dest.is_null() {
        println!("failed to open for write: {}", args.get(2));
        exit(1);
    }

    let mut buf = [0_u8; 256];
    loop {
        let bytes = read_file2(fp_src, &mut buf);
        if bytes <= 0 {
            break;
        }
        let written = write_file(fp_dest, &buf[bytes..]);
        println!("bytes {}, written = {}", bytes, written);
        if bytes != written {
            println!("failed to write to {}", args.get(2));
            exit(1);
        }
    }

    exit(0)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
