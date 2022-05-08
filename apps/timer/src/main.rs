#![no_std]
#![no_main]
#![feature(format_args_nl)]

use core::arch::asm;
use core::panic::PanicInfo;
use core::str::FromStr;
use shared_lib::app_event::AppEventType;
use shared_lib::args::Args;
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{create_timer, println, read_event, TimerType};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    if argc <= 1 {
        println!("Usage: timer <msec>");
        exit(1);
    }
    let args = Args::new(argc, argv);

    let duration_ms = u64::from_str(args.get(1)).unwrap();
    let timeout = match create_timer(TimerType::OneshotRel, 1, duration_ms) {
        Ok(t) => t,
        Err(e) => exit(e.error_number()),
    };
    println!("timer created, timeout = {}", timeout);

    let mut events = [Default::default(); 1];
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
            // AppEventType::Quit => break,
            AppEventType::TimerTimeout => {
                println!("{} ms elapsed!", duration_ms);
                break;
            }
            _ => println!("unknown event: type = {:?}", events[0].type_),
        }
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
