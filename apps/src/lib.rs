#![no_std]

use crate::rust_official::cchar::c_char;

pub mod rust_official;

extern "C" {
    pub fn strlen(cs: *const c_char) -> usize;
}
