use crate::{memory_manager, FrameID, BYTES_PER_FRAME};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use log::error;

pub struct MemoryAllocator;

unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let num_frames = (layout.size() + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME;
        match memory_manager().allocate(num_frames) {
            Ok(frame) => (frame.id() * BYTES_PER_FRAME) as *mut u8,
            Err(e) => {
                error!("an error occurs when allocating {:?}: {}", layout, e);
                null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let start_frame = FrameID::new(ptr as usize / BYTES_PER_FRAME);
        let num_frames = (layout.size() + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME;
        memory_manager().free(start_frame, num_frames).unwrap();
    }
}
