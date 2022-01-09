#![no_std]

use core::ffi::c_void;

#[derive(Eq, PartialEq)]
#[repr(C)]
pub enum PixelFormat {
    KPixelRGBResv8BitPerColor,
    KPixelBGRResv8BitPerColor,
}

#[derive(Eq, PartialEq)]
#[warn(dead_code)]
#[repr(C)]
pub struct FrameBufferConfig {
    pub frame_buffer: *mut u8,
    pub pixels_per_scan_line: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: PixelFormat,
}

#[warn(dead_code)]
#[repr(C)]
#[derive(Debug)]
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

// doesn't generate code without this method...
#[no_mangle]
pub unsafe extern "C" fn _dummy(
    _: *const FrameBufferConfig,
    _: MemoryMap,
    _: MemoryDescriptor,
) {}