#![no_std]

#[derive(Eq, PartialEq)]
#[repr(C)]
pub enum PixelFormat {
    KPixelRGBResv8bitPerColor,
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

// doesn't generate code without this method...
#[no_mangle]
pub unsafe extern "C" fn _dummy(_: *const FrameBufferConfig) {}
