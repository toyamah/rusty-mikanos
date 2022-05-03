#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use core::str::FromStr;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use shared_lib::args::Args;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::window::Window;
use shared_lib::{current_tick_millis, println};

const WIDTH: i32 = 100;
const HEIGHT: i32 = 100;

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let mut window = match Window::open((WIDTH, HEIGHT), (10, 10), "stars") {
        Ok(w) => w,
        Err(e) => exit(e.error_number()),
    };

    window.fill_rectangle((4, 24), (WIDTH, HEIGHT), 0x000000);

    let num_stars = if args.len() <= 1 {
        100
    } else {
        usize::from_str(args.get(1)).unwrap()
    };

    let tick_start = current_tick_millis();

    let mut rng = SmallRng::from_seed([0; 32]);
    for _ in 0..num_stars {
        let x: i32 = rng.gen_range(0..WIDTH - 2);
        let y: i32 = rng.gen_range(0..HEIGHT - 2);
        window.fill_rectangle((4 + x, 24 + y), (2, 2), 0xfff100);
    }

    println!(
        "{} stars in {} ms.",
        num_stars,
        current_tick_millis() - tick_start
    );

    exit(0)
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
