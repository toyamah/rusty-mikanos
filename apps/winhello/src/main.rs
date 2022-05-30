#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use shared_lib::app_event::AppEventType;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::{Window, FLAG_FORCE_DRAW};
use shared_lib::{println, read_event};

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((200, 100), (10, 10), "winhello") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    w.write_string((7, 24), 0xc00000, "hello world!", FLAG_FORCE_DRAW);
    w.write_string((24, 40), 0x00c000, "hello world!", FLAG_FORCE_DRAW);
    w.write_string((40, 56), 0x0000c0, "hello world!", FLAG_FORCE_DRAW);

    let mut events = [Default::default(); 1];
    loop {
        match read_event(events.as_mut(), 1) {
            Ok(_) => {}
            Err(e) => {
                println!("ReadEvent failed: {}", e.strerror());
            }
        };
        match events[0].type_ {
            AppEventType::Quit => break,
            _ => println!("unknown event: type = {:?}", events[0].type_),
        }
    }

    w.close();
    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
