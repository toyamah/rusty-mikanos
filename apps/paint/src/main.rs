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

const WIDTH: i32 = 200;
const HEIGHT: i32 = 130;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((200, 100), (10, 10), "paint") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    let mut events = [Default::default(); 1];
    let mut press = false;
    loop {
        match read_event(events.as_mut(), 1) {
            Ok(_) => {}
            Err(e) => {
                println!("ReadEvent failed: {}", e.strerror());
                break;
            }
        };
        let event = &events[0];
        match event.type_ {
            AppEventType::Quit => break,
            AppEventType::MouseMove => {
                let arg = unsafe { event.arg.mouse_move };
                let prev_x = arg.x - arg.dx;
                let prev_y = arg.y - arg.dy;
                if press && is_inside(prev_x, prev_y) && is_inside(arg.x, arg.y) {
                    w.draw_line(prev_x, prev_y, arg.x, arg.y, 0x000000);
                }
            }
            AppEventType::MouseButton => {
                let arg = unsafe { event.arg.mouse_button };
                if arg.button == 0 {
                    press = arg.is_pressed();
                    w.fill_rectangle((arg.x, arg.y), (1, 1), 0x000000, FLAG_FORCE_DRAW);
                }
            }
            _ => println!("unknown event: type = {:?}", events[0].type_),
        }
    }

    w.close();
    exit(0);
}

fn is_inside(x: i32, y: i32) -> bool {
    4 <= x && x < 4 + WIDTH && 24 <= y && y < 24 + HEIGHT
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
