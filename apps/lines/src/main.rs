#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::f64::consts::PI;
use core::panic::PanicInfo;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::Window;

const RADIUS: i32 = 90;

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const c_char) {
    let mut window = match Window::open((RADIUS * 2 + 10, RADIUS), (10, 10), "lines") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    let x0 = 4;
    let y0 = 24;
    let x1 = 4 + RADIUS;
    let y1 = 24 + RADIUS;
    let radius = RADIUS as f64;

    for deq in (0..90).step_by(5) {
        let x = (radius * libm::cos(PI * deq as f64 / 180.0)) as i32;
        let y = (radius * libm::sin(PI * deq as f64 / 180.0)) as i32;
        window.draw_line(x0, y0, x0 + x, y0 + y, color(deq));
        window.draw_line(x1, y1, x1 + x, y1 - y, color(deq));
    }

    exit(0);
}

fn color(deg: u32) -> u32 {
    if deg <= 30 {
        (255 * deg / 30 << 8) | 0xff0000
    } else if deg <= 60 {
        (255 * (60 - deg) / 30) << 16 | 0x00ff00
    } else if deg <= 90 {
        (255 * (deg - 60) / 30) | 0x00ff00
    } else if deg <= 120 {
        (255 * (120 - deg) / 30) << 8 | 0x0000ff
    } else if deg <= 150 {
        (255 * (deg - 120) / 30) << 16 | 0x0000ff
    } else {
        (255 * (180 - deg) / 30) | 0xff0000
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
