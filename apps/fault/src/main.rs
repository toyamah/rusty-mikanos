#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::newlib_support::exit;
use shared_lib::println;
use shared_lib::rust_official::cchar::c_char;

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let cmd = if args.len() >= 2 { args.get(1) } else { "hlt" };

    match cmd {
        "hlt" => unsafe { asm!("hlt") },
        "wr_kernel" => {
            let p = 0x100 as *mut i32;
            unsafe { *p = 43 }
        }
        "wr_app" => {
            let p = 0xffff8000ffff0000 as *mut i32;
            unsafe { *p = 123 }
        }
        "zero" => {
            let z = 0;
            println!("100/{} = {}", z, 100 / z)
        }
        _ => {}
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
