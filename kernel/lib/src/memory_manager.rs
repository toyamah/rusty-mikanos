use crate::error::{Code, Error};
use crate::make_error;
use core::ffi::c_void;

pub mod global {
    use super::{BitmapMemoryManager, FrameID, BYTES_PER_FRAME};
    use crate::memory_map::UEFI_PAGE_SIZE;
    use crate::sync::Mutex;
    use shared::{MemoryDescriptor, MemoryMap};

    pub static MEMORY_MANAGER: Mutex<BitmapMemoryManager> = Mutex::new(BitmapMemoryManager::new());

    pub fn initialize(memory_map: &MemoryMap) {
        let buffer = memory_map.buffer as usize;
        let mut available_end = 0;
        let mut iter = buffer;
        while iter < buffer + memory_map.map_size as usize {
            let desc = iter as *const MemoryDescriptor;
            let physical_start = unsafe { (*desc).physical_start };
            let number_of_pages = unsafe { (*desc).number_of_pages };
            if available_end < physical_start {
                MEMORY_MANAGER.lock().mark_allocated(
                    FrameID::new(available_end / BYTES_PER_FRAME),
                    (physical_start - available_end) / BYTES_PER_FRAME,
                );
            }

            let type_ = unsafe { &(*desc).type_ };
            let byte_count = (number_of_pages * UEFI_PAGE_SIZE as u64) as usize;
            let physical_end = physical_start + byte_count;
            if type_.is_available() {
                available_end = physical_end;
            } else {
                MEMORY_MANAGER.lock().mark_allocated(
                    FrameID::new(physical_start / BYTES_PER_FRAME),
                    byte_count / BYTES_PER_FRAME as usize,
                )
            }
            iter += memory_map.descriptor_size as usize;
        }
        MEMORY_MANAGER.lock().set_memory_range(
            FrameID::new(1),
            FrameID::new(available_end / BYTES_PER_FRAME),
        );
    }
}

const fn kib(kib: u64) -> u64 {
    kib * 1024
}

const fn mib(mib: u64) -> u64 {
    mib * kib(1024)
}

const fn gib(gib: u64) -> u64 {
    gib * mib(1024)
}

//
// FrameId
//

pub const BYTES_PER_FRAME: usize = kib(4) as usize;
// const NULL_FRAME: FrameID = FrameID(usize::MAX);

#[derive(Copy, Clone, Debug)]
pub struct FrameID(usize);

impl FrameID {
    pub fn new(v: usize) -> FrameID {
        Self(v)
    }

    pub fn id(&self) -> usize {
        self.0
    }

    pub fn frame(&self) -> *const c_void {
        (self.0 * BYTES_PER_FRAME as usize) as *const c_void
    }
}

pub struct MemoryStat {
    pub allocated_frames: usize,
    pub total_frames: usize,
}

impl MemoryStat {
    fn new(allocated_frames: usize, total_frames: usize) -> Self {
        Self {
            allocated_frames,
            total_frames,
        }
    }

    pub fn calc_allocated_size_in_mb(&self) -> usize {
        self.allocated_frames * BYTES_PER_FRAME / 1024 / 1024
    }

    pub fn calc_total_size_in_mb(&self) -> usize {
        self.total_frames * BYTES_PER_FRAME / 1024 / 1024
    }
}

//
// BitmapMemoryManager
//
type MapLineType = u64;

const MAX_PHYSICAL_MEMORY_BYTES: usize = gib(128) as usize;
const FRAME_COUNT: usize = MAX_PHYSICAL_MEMORY_BYTES / BYTES_PER_FRAME;
const BITS_PER_MAP_LINE: usize = 8 * core::mem::size_of::<MapLineType>();

pub struct BitmapMemoryManager {
    pub alloc_map: [u64; FRAME_COUNT / BITS_PER_MAP_LINE],
    pub range_begin: FrameID,
    pub range_end: FrameID,
}

impl BitmapMemoryManager {
    pub const fn new() -> BitmapMemoryManager {
        Self {
            alloc_map: [0; FRAME_COUNT / BITS_PER_MAP_LINE],
            range_begin: FrameID(0),
            range_end: FrameID(FRAME_COUNT),
        }
    }

    pub fn allocate(&mut self, num_frames: usize) -> Result<FrameID, Error> {
        let mut start_frame_id = self.range_begin.id();
        loop {
            let mut i: usize = 0;
            while i < num_frames {
                if start_frame_id + i >= self.range_end.id() {
                    return Err(make_error!(Code::NoEnoughMemory));
                }
                if self.get_bit(FrameID(start_frame_id + i)) {
                    break;
                }
                i += 1;
            }

            if i == num_frames {
                self.mark_allocated(FrameID(start_frame_id), num_frames);
                return Ok(FrameID(start_frame_id));
            }

            start_frame_id += i + 1;
        }
    }

    pub fn free(&mut self, start_frame: FrameID, num_frames: usize) -> Result<(), Error> {
        for i in 0..num_frames {
            self.set_bit(FrameID(start_frame.0 + i), false);
        }
        Ok(())
    }

    pub fn mark_allocated(&mut self, start_frame: FrameID, num_frames: usize) {
        for i in 0..num_frames {
            self.set_bit(FrameID(start_frame.0 + i), true);
        }
    }

    pub fn set_memory_range(&mut self, range_begin: FrameID, range_end: FrameID) {
        self.range_begin = range_begin;
        self.range_end = range_end;
    }

    pub fn stat(&self) -> MemoryStat {
        let start = self.range_begin.id() / BITS_PER_MAP_LINE;
        let end = self.range_end.id() / BITS_PER_MAP_LINE;
        let sum = (start..end).fold(0, |acc, i| acc + self.alloc_map[i].count_ones() as usize);
        MemoryStat::new(sum, self.range_end.id() - self.range_begin.id())
    }

    fn get_bit(&self, frame: FrameID) -> bool {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;
        (self.alloc_map[line_index] & 1 << bit_index) != 0
    }

    fn set_bit(&mut self, frame: FrameID, allocated: bool) {
        let line_index = frame.id() / BITS_PER_MAP_LINE;
        let bit_index = frame.id() % BITS_PER_MAP_LINE;

        if allocated {
            self.alloc_map[line_index] |= 1 << bit_index;
        } else {
            self.alloc_map[line_index] &= !(1 << bit_index);
        }
    }
}
