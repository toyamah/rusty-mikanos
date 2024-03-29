#![feature(format_args_nl)]
#![feature(alloc_error_handler)]
#![no_std]

use crate::app_event::AppEvent;
use crate::byte_buffer::ByteBuffer;
use crate::newlib_support::write;
use crate::rust_official::cchar::c_char;
use crate::syscall::{
    SyscallCreateTimer, SyscallDemandPages, SyscallError, SyscallGetCurrentTick, SyscallLogString,
    SyscallOpenWindow, SyscallReadEvent, SyscallWinWriteString,
};
use core::ffi::c_void;
use core::fmt;

mod allocator;
pub mod app_event;
pub mod args;
mod byte_buffer;
pub mod file;
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

pub fn current_tick_millis() -> u64 {
    let result = unsafe { SyscallGetCurrentTick() };
    let timer_freq = result.error;
    result.value * 1000 / timer_freq as u64
}

pub fn read_event(events: &mut [AppEvent], len: usize) -> Result<u64, SyscallError> {
    unsafe { SyscallReadEvent(events.as_ptr() as *const c_void, len) }.to_result()
}

pub fn demand_page(num_pages: usize, flags: i32) -> Result<u64, SyscallError> {
    unsafe { SyscallDemandPages(num_pages, flags) }.to_result()
}

pub enum TimerType {
    OneshotRel = 1,
    OneshotAbs = 0,
}

pub fn create_timer(
    type_: TimerType,
    timer_value: i32,
    timeout_ms: u64,
) -> Result<u64, SyscallError> {
    unsafe { SyscallCreateTimer(type_ as u64, timer_value, timeout_ms) }.to_result()
}

#[macro_export]
macro_rules! println {
    () => ($crate::print("\n"));
    ($($arg:tt)*) => ({
        $crate::printf(format_args_nl!($($arg)*));
    })
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::printf(format_args!($($arg)*)));
}
