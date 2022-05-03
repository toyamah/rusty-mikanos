#![feature(format_args_nl)]
#![no_std]

use crate::byte_buffer::ByteBuffer;
use crate::newlib_support::write;
use crate::rust_official::cchar::c_char;
use crate::syscall::{SyscallError, SyscallLogString, SyscallOpenWindow, SyscallWinWriteString};
use core::ffi::c_void;
use core::fmt;

pub mod args;
mod byte_buffer;
pub mod libc;
pub mod newlib_support;
pub mod rust_official;
mod syscall;
pub mod window;

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

#[macro_export]
macro_rules! println {
    () => ($crate::print("\n"));
    ($($arg:tt)*) => ({
        $crate::printf(format_args_nl!($($arg)*));
    })
}
