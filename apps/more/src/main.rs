#![no_std]
#![no_main]
#![feature(format_args_nl)]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use core::arch::asm;
use core::panic::PanicInfo;
use core::str::FromStr;
use shared_lib::app_event::AppEventType;
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_string, OpenMode};
use shared_lib::newlib_support::{exit, FILE};
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println, read_event};

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let (page_size, input) = convert_args(Args::new(argc, argv));

    let mut lines = vec![];
    let mut line = [0_u8; 256];
    while !read_string(input, line.as_mut()).is_null() {
        let str = buf_to_str(&line).unwrap();
        lines.push(str.to_string());
    }

    for (i, string) in lines.iter().enumerate() {
        if i > 0 && (i % page_size) == 0 {
            println!("--more--");
            wait_key();
        }
        print!("{}", string);
    }

    exit(0)
}

fn convert_args(args: Args) -> (usize, *mut FILE) {
    let (page_size, arg_file) = match convert_to_page_size(&args) {
        None => (10, 1),
        Some(page_size) => (page_size, 2),
    };

    let path = if args.len() > arg_file {
        args.get(arg_file)
    } else {
        "@stdin"
    };
    let input = open_file(path, OpenMode::R);
    if input.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    (page_size, input)
}

fn convert_to_page_size(args: &Args) -> Option<usize> {
    if args.len() < 2 {
        return None;
    }

    let arg = args.get(1);
    if !arg.starts_with('-') {
        return None;
    }

    usize::from_str(&arg[1..]).ok()
}

fn wait_key() {
    let mut events = [Default::default(); 1];
    loop {
        match read_event(events.as_mut(), 1) {
            Ok(_) => {}
            Err(e) => {
                println!("ReadEvent failed: {}", e.strerror());
                exit(1)
            }
        };

        let event = &events[0];
        match event.type_ {
            AppEventType::Quit => exit(0),
            AppEventType::KeyPush => {
                let arg = unsafe { event.arg.key_push };
                if arg.press {
                    return;
                }
            }
            _ => println!("unknown event: type = {:?}", events[0].type_),
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
