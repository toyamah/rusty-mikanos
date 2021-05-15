use shared::FrameBufferConfig;

pub struct PixelColor {
    r: u8,
    g: u8,
    b: u8,
}

impl PixelColor {
    pub fn new(r: u8, g: u8, b: u8) -> PixelColor {
        PixelColor { r, g, b }
    }
}

pub trait PixelWriter {
    fn write(self: &Self, x: u32, y: u32, color: &PixelColor);
}

pub struct RGBResv8BitPerColorPixelWriter<'a> {
    config: &'a FrameBufferConfig,
}

impl<'a> RGBResv8BitPerColorPixelWriter<'a> {
    pub fn new(config: &'a FrameBufferConfig) -> RGBResv8BitPerColorPixelWriter {
        Self { config }
    }
}

impl<'a> PixelWriter for RGBResv8BitPerColorPixelWriter<'a> {
    fn write(self: &Self, x: u32, y: u32, color: &PixelColor) {
        let p = pixel_at(x, y, self.config);
        unsafe {
            *p.offset(0) = color.r;
            *p.offset(1) = color.g;
            *p.offset(2) = color.b;
        }
    }
}

pub struct BGRResv8BitPerColorPixelWriter<'a> {
    config: &'a FrameBufferConfig,
}

impl<'a> BGRResv8BitPerColorPixelWriter<'a> {
    pub fn new(config: &'a FrameBufferConfig) -> BGRResv8BitPerColorPixelWriter<'a> {
        Self { config }
    }
}

impl<'a> PixelWriter for BGRResv8BitPerColorPixelWriter<'a> {
    fn write(self: &Self, x: u32, y: u32, color: &PixelColor) {
        let p = pixel_at(x, y, self.config);
        unsafe {
            *p.offset(0) = color.b;
            *p.offset(1) = color.g;
            *p.offset(2) = color.r;
        }
    }
}

fn pixel_at(x: u32, y: u32, config: &FrameBufferConfig) -> *mut u8 {
    let pixel_position = config.pixels_per_scan_line * y + x;
    let base = (4 * pixel_position) as isize;
    unsafe { config.frame_buffer.offset(base) }
}
