use crate::memory_manager;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use lib::memory_manager::{FrameID, BYTES_PER_FRAME};
use log::error;

pub struct MemoryAllocator;

// I'm not sure that this implementation is correct especially about memory alignment...
// But seems like it works.
unsafe impl GlobalAlloc for MemoryAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let num_frames = (layout.size() + layout.align() + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME;
        match memory_manager().allocate(num_frames) {
            Ok(frame) => (frame.id() * BYTES_PER_FRAME + layout.align()) as *mut u8,
            Err(e) => {
                error!("an error occurs when allocating {:?}: {}", layout, e);
                null_mut()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let start_frame = FrameID::new(ptr as usize / BYTES_PER_FRAME);
        let num_frames = (layout.size() + layout.align() + BYTES_PER_FRAME - 1) / BYTES_PER_FRAME;
        memory_manager().free(start_frame, num_frames).unwrap();
    }
}
