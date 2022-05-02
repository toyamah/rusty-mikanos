#![feature(format_args_nl)]
#![no_std]

use crate::byte_buffer::ByteBuffer;
use crate::newlib_support::write;
use crate::rust_official::cchar::c_char;
use crate::syscall::{SyscallLogString, SyscallOpenWindow};
use core::ffi::c_void;
use core::fmt;

mod byte_buffer;
pub mod newlib_support;
pub mod rust_official;
mod syscall;

extern "C" {
    pub fn strlen(cs: *const c_char) -> usize;
    pub fn atol(s: *const c_char) -> i64;
    // pub fn strcmp(a: *const c_char, b: *const c_char) -> i32;
}

pub fn print(s: &str) {
    write(1, s.as_ptr() as *const c_void, s.as_bytes().len());
}

pub fn printf(args: fmt::Arguments) {
    let mut buf = ByteBuffer::new();
    fmt::write(&mut buf, args).expect("failed to write ByteBuffer");
    write(1, buf.as_ptr_void(), buf.len());
}

pub fn info(s: &str) {
    unsafe {
        SyscallLogString(3, s.as_ptr() as *const c_char);
    }
}

pub fn open_window(w: i32, h: i32, x: i32, y: i32, title: &str) {
    let mut buf = ByteBuffer::new();
    buf.write_str_with_nul(title);
    unsafe { SyscallOpenWindow(w, h, x, y, buf.as_ptr_c_char()) };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print("\n"));
    ($($arg:tt)*) => ({
        $crate::printf(format_args_nl!($($arg)*));
    })
}
