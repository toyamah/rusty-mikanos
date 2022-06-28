#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::string::ToString;
use core::alloc::{GlobalAlloc, Layout};
use core::arch::asm;
use core::ffi::c_void;
use core::panic::PanicInfo;
use shared_lib::newlib_support::exit;
use shared_lib::println;
use shared_lib::rust_official::cchar::c_char;

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
    let a = "a".to_string();
    println!("{}", a);
    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
