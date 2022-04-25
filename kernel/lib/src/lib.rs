#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![feature(const_btree_new)]
#![feature(slice_internals)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use crate::window::Window;
pub mod acpi;
pub mod asm;
pub mod console;
mod elf;
pub mod error;
pub mod fat;
mod font;
pub mod frame_buffer;
pub mod graphics;
pub mod interrupt;
pub mod keyboard;
pub mod layer;
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
