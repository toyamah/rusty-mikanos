#![cfg_attr(not(test), no_std)]

extern crate alloc;

use crate::window::Window;
pub mod asm;
pub mod error;
mod font;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod memory_manager;
pub mod memory_map;
pub mod mouse;
pub mod paging;
pub mod pci;
pub mod queue;
pub mod segment;
pub mod window;
mod x86_descriptor;

mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
