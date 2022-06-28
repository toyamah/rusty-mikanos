#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::string::ToString;
use alloc::vec;
use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use core::ffi::c_void;
use core::panic::PanicInfo;
use shared_lib::args::Args;
use shared_lib::file::{buf_to_str, open_file, read_string, OpenMode};
use shared_lib::newlib_support::exit;
use shared_lib::rust_official::cchar::c_char;
use shared_lib::{print, println};

#[global_allocator]
static ALLOCATOR: MemoryAllocator = MemoryAllocator;

#[alloc_error_handler]
fn alloc_error_handle(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free<'a>(p: *mut c_void);
}

pub struct MemoryAllocator;

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        free(ptr as *mut c_void)
    }
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) {
    let args = Args::new(argc, argv);
    let path = if args.len() >= 2 {
        args.get(1)
    } else {
        "@stdin"
    };

    let fp = open_file(path, OpenMode::R);
    if fp.is_null() {
        println!("failed to open {}", path);
        exit(1);
    }

    let mut line = [0_u8; 1024];
    let mut lines = vec![];
    while !read_string(fp, &mut line).is_null() {
        lines.push(buf_to_str(&line).unwrap().to_string());
    }

    lines.sort();

    for line in lines {
        print!("{}", line);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
