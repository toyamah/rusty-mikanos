#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![feature(const_btree_new)]
#![feature(slice_internals)]
#![feature(abi_x86_interrupt)]
#![feature(map_first_last)]

extern crate alloc;

use crate::window::Window;
use core::str::Utf8Error;
pub mod acpi;
mod app_event;
pub mod asm;
pub mod console;
mod elf;
pub mod error;
pub mod fat;
pub mod font;
pub mod frame_buffer;
pub mod graphics;
pub mod interrupt;
mod io;
pub mod keyboard;
pub mod layer;
mod libc;
pub mod memory_manager;
pub mod memory_map;
pub mod message;
pub mod mouse;
mod msr;
pub mod paging;
pub mod pci;
mod rust_official;
pub mod segment;
pub mod syscall;
pub mod task;
pub mod terminal;
pub mod timer;
pub mod window;
mod x86_descriptor;

pub(crate) fn str_trimming_nul(buf: &[u8]) -> Result<&str, Utf8Error> {
    let nul_term_index = buf
        .iter()
        .enumerate()
        .find(|(_, &b)| b == 0)
        .map(|(i, _)| i)
        .unwrap_or(buf.len());
    core::str::from_utf8(&buf[..nul_term_index])
}

pub(crate) fn str_trimming_nul_unchecked(buf: &[u8]) -> &str {
    str_trimming_nul(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_trimming_nul_unchecked_test() {
        assert_eq!(str_trimming_nul_unchecked(b"\0"), "");
        assert_eq!(str_trimming_nul_unchecked(b"abc\0"), "abc");
        assert_eq!(str_trimming_nul_unchecked(b"bcd\0\0\0"), "bcd");
        assert_eq!(str_trimming_nul_unchecked(b"\0cde\0\0\0"), "");
    }
}
