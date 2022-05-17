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
