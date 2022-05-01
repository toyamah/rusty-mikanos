#![no_std]

use crate::rust_official::cchar::c_char;

pub mod rust_official;

extern "C" {
    pub fn strlen(cs: *const c_char) -> usize;
    pub fn atol(s: *const c_char) -> i64;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> i32;
}
