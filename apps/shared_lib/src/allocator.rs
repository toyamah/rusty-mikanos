use crate::libc::{free, malloc};
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

#[global_allocator]
static ALLOCATOR: MemoryAllocator = MemoryAllocator;

#[alloc_error_handler]
fn alloc_error_handle(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub struct MemoryAllocator;

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        free(ptr as *mut c_void)
    }
}
