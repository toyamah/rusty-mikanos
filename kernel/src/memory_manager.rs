use crate::error::Code;
use crate::{make_error, printk, Error};
use core::ffi::c_void;

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

pub const BYTES_PER_FRAME: u64 = kib(4);
const NULL_FRAME: FrameID = FrameID(usize::MAX);

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

//
// BitmapMemoryManager
//
type MapLineType = u64;

const MAX_PHYSICAL_MEMORY_BYTES: u64 = gib(128);
const FRAME_COUNT: usize = (MAX_PHYSICAL_MEMORY_BYTES / BYTES_PER_FRAME) as usize;
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
