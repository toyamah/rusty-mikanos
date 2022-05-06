#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use libm::{atan2, cos, fmin, pow, sin, sqrt};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::{Window, FLAG_NO_DRAW};
use shared_lib::{println, read_event, Type};

const CANVAS_SIZE: i32 = 100;
const EYE_SIZE: i32 = 10;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut w = match Window::open((CANVAS_SIZE, CANVAS_SIZE), (10, 10), "eye") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    w.fill_rectangle((4, 24), (CANVAS_SIZE, CANVAS_SIZE), 0xffffff, 0);

    let mut events = [Default::default(); 1];
    loop {
        match read_event(events.as_mut(), 1) {
            Ok(_) => {}
            Err(e) => {
                println!("ReadEvent failed: {}", e.strerror());
            }
        };

        let event = &events[0];
        match event.type_() {
            Type::Quit => break,
            Type::MouseMove => {
                let arg = unsafe { event.arg.mouse_move };
                w.fill_rectangle((4, 24), (CANVAS_SIZE, CANVAS_SIZE), 0xffffff, FLAG_NO_DRAW);
                draw_eye(&mut w, arg.x, arg.y, 0x000000);
            }
            _ => println!("unknown event: type = {:?}", events[0].type_()),
        }
    }

    w.close();
    exit(0)
}

fn draw_eye(window: &mut Window, mouse_x: i32, mouse_y: i32, color: u32) {
    let canvas_size = CANVAS_SIZE as f64;
    let eye_size = EYE_SIZE as f64;
    let center_x = mouse_x as f64 - canvas_size / 2.0 - 4.0;
    let center_y = mouse_y as f64 - canvas_size / 2.0 - 24.0;

    let direction = atan2(center_y, center_x);
    let distance = sqrt(pow(center_x, 2.0) + pow(center_y, 2.0));
    let distance = fmin(distance, canvas_size / 2.0 - eye_size / 2.0);

    let eye_center_x = cos(direction) * distance;
    let eye_center_y = sin(direction) * distance;
    let eye_x = eye_center_x as i32 + CANVAS_SIZE / 2 + 4;
    let eye_y = eye_center_y as i32 + CANVAS_SIZE / 2 + 24;

    window.fill_rectangle(
        (eye_x - EYE_SIZE / 2, eye_y - EYE_SIZE / 2),
        (EYE_SIZE, EYE_SIZE),
        color,
        0,
    );
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
