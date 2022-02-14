#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

extern crate alloc;

use crate::window::Window;
pub mod asm;
pub mod error;
mod font;
pub mod frame_buffer;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod memory_manager;
pub mod memory_map;
pub mod message;
pub mod mouse;
pub mod paging;
pub mod pci;
pub mod segment;
pub mod timer;
pub mod window;
mod x86_descriptor;
pub mod console;
