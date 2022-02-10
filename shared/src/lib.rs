#![no_std]

use core::ffi::c_void;

#[derive(Eq, PartialEq, Clone, Copy)]
#[repr(C)]
pub enum PixelFormat {
    KPixelRGBResv8BitPerColor,
    KPixelBGRResv8BitPerColor,
}

impl PixelFormat {
    pub fn bits_per_pixel(&self) -> usize {
        match self {
            PixelFormat::KPixelRGBResv8BitPerColor => 32,
            PixelFormat::KPixelBGRResv8BitPerColor => 32,
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        let bits = self.bits_per_pixel();
        (bits + 7) / 8
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
#[warn(dead_code)]
#[repr(C)]
pub struct FrameBufferConfig {
    pub frame_buffer: *mut u8,
    pub pixels_per_scan_line: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: PixelFormat,
}

impl FrameBufferConfig {
    pub fn new(
        horizontal_resolution: u32,
        vertical_resolution: u32,
        pixels_per_scan_line: u32,
        pixel_format: PixelFormat,
    ) -> FrameBufferConfig {
        Self {
            frame_buffer: core::ptr::null_mut(),
            pixels_per_scan_line,
            horizontal_resolution,
            vertical_resolution,
            pixel_format,
        }
    }

    pub fn bytes_per_scan_line(&self) -> usize {
        self.pixel_format.bytes_per_pixel() * self.pixels_per_scan_line as usize
    }

    pub fn pixel_position_at(&self, x: usize, y: usize) -> usize {
        self.pixel_format.bytes_per_pixel() * (self.pixels_per_scan_line as usize * y + x)
    }

    pub unsafe fn frame_addr_at(&self, x: usize, y: usize) -> *mut u8 {
        self.frame_buffer.add(self.pixel_position_at(x, y))
    }
}

/// To generate unsigned long long type, each value should be defined as c_ulonglong.
/// However, c_longlong is in the std library which cannot be used here.
/// To solve this, I decided to keep MemoryMap from generating to shared_header.h and left the original MemoryMap type in Main.c.
/// https://github.com/eqrion/cbindgen/blob/master/docs.md#libc-types
#[warn(dead_code)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    pub buffer_size: u64,
    pub buffer: *mut c_void,
    pub map_size: u64,
    pub map_key: u64,
    pub descriptor_size: u64,
    pub descriptor_version: u32,
}

#[derive(Eq, PartialEq)]
#[warn(dead_code)]
#[repr(C)]
pub struct MemoryDescriptor {
    pub type_: MemoryType,
    pub physical_start: usize,
    pub virtual_start: usize,
    pub number_of_pages: u64,
    pub attribute: u64,
}

// not adding it to _dummy function to ignore generating to shared_header.h
#[warn(dead_code)]
#[derive(Eq, PartialEq, Debug)]
#[repr(C)]
pub enum MemoryType {
    KEfiReservedMemoryType,
    KEfiLoaderCode,
    KEfiLoaderData,
    KEfiBootServicesCode,
    KEfiBootServicesData,
    KEfiRuntimeServicesCode,
    KEfiRuntimeServicesData,
    KEfiConventionalMemory,
    KEfiUnusableMemory,
    KEfiACPIReclaimMemory,
    KEfiACPIMemoryNVS,
    KEfiMemoryMappedIO,
    KEfiMemoryMappedIOPortSpace,
    KEfiPalCode,
    KEfiPersistentMemory,
    KEfiMaxMemoryType,
}

impl MemoryType {
    pub fn to_i32(&self) -> i32 {
        return match self {
            MemoryType::KEfiReservedMemoryType => 0,
            MemoryType::KEfiLoaderCode => 1,
            MemoryType::KEfiLoaderData => 2,
            MemoryType::KEfiBootServicesCode => 3,
            MemoryType::KEfiBootServicesData => 4,
            MemoryType::KEfiRuntimeServicesCode => 5,
            MemoryType::KEfiRuntimeServicesData => 6,
            MemoryType::KEfiConventionalMemory => 7,
            MemoryType::KEfiUnusableMemory => 8,
            MemoryType::KEfiACPIReclaimMemory => 9,
            MemoryType::KEfiACPIMemoryNVS => 10,
            MemoryType::KEfiMemoryMappedIO => 11,
            MemoryType::KEfiMemoryMappedIOPortSpace => 12,
            MemoryType::KEfiPalCode => 13,
            MemoryType::KEfiPersistentMemory => 14,
            MemoryType::KEfiMaxMemoryType => 15,
        };
    }

    pub fn is_available(&self) -> bool {
        return match self {
            MemoryType::KEfiBootServicesCode
            | MemoryType::KEfiBootServicesData
            | MemoryType::KEfiConventionalMemory => true,
            MemoryType::KEfiReservedMemoryType
            | MemoryType::KEfiLoaderCode
            | MemoryType::KEfiLoaderData
            | MemoryType::KEfiRuntimeServicesCode
            | MemoryType::KEfiRuntimeServicesData
            | MemoryType::KEfiUnusableMemory
            | MemoryType::KEfiACPIReclaimMemory
            | MemoryType::KEfiACPIMemoryNVS
            | MemoryType::KEfiMemoryMappedIO
            | MemoryType::KEfiMemoryMappedIOPortSpace
            | MemoryType::KEfiPalCode
            | MemoryType::KEfiPersistentMemory
            | MemoryType::KEfiMaxMemoryType => false,
        };
    }
}

// doesn't generate code without this method...
#[no_mangle]
pub unsafe extern "C" fn _dummy(
    _: *const FrameBufferConfig,
    // _: MemoryMap,
    _: MemoryDescriptor,
) {
}
