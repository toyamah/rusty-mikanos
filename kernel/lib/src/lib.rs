#![cfg_attr(not(test), no_std)]

extern crate alloc;

use crate::console::Console;
use crate::layer::LayerManager;
use crate::window::Window;

pub mod asm;
pub mod console;
pub mod error;
mod font;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod logger;
pub mod memory_manager;
pub mod memory_map;
pub mod mouse;
pub mod paging;
pub mod pci;
pub mod queue;
pub mod segment;
pub mod usb;
pub mod window;
mod x86_descriptor;

pub static mut CONSOLE: Option<Console> = None;

pub fn console() -> &'static mut Console<'static> {
    unsafe { CONSOLE.as_mut().unwrap() }
}

pub static mut LAYER_MANAGER: Option<LayerManager> = None;
pub fn layer_manager_op() -> Option<&'static mut LayerManager<'static>> {
    unsafe { LAYER_MANAGER.as_mut() }
}
pub fn layer_manager() -> &'static mut LayerManager<'static> {
    unsafe { LAYER_MANAGER.as_mut().unwrap() }
}

mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
