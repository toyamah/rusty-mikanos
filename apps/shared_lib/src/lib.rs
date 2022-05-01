#![no_std]

use crate::newlib_support::{write, CallResult};
use crate::rust_official::cchar::c_char;
use core::ffi::c_void;

pub mod newlib_support;
pub mod rust_official;

extern "C" {
    pub fn strlen(cs: *const c_char) -> usize;
    pub fn atol(s: *const c_char) -> i64;
    // pub fn strcmp(a: *const c_char, b: *const c_char) -> i32;

    pub(crate) fn SyscallLogString(level: i64, s: *const c_char) -> CallResult;
}

pub fn print(s: &str) {
    write(1, s.as_ptr() as *const c_void, s.as_bytes().len());
}

pub fn info(s: &str) {
    unsafe {
        SyscallLogString(3, s.as_ptr() as *const c_char);
    }
}
