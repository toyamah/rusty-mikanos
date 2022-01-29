use crate::graphics::{FrameBufferWriter, Vector2D};
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::{max, min};
use core::ptr::copy_nonoverlapping;
use shared::FrameBufferConfig;

pub struct FrameBuffer {
    config: FrameBufferConfig,
    buffer: Vec<u8>,
    writer: FrameBufferWriter,
}

impl FrameBuffer {
    pub fn new(mut config: FrameBufferConfig) -> Self {
        let buffer = if config.frame_buffer.is_null() {
            let cap = config.pixel_format.bytes_per_pixel()
                * config.horizontal_resolution as usize
                * config.vertical_resolution as usize;
            let mut buf: Vec<u8> = (0..cap).map(|_| 0).collect();
            config.frame_buffer = buf.as_mut_ptr();
            config.pixels_per_scan_line = config.horizontal_resolution;
            buf
        } else {
            vec![]
        };
        Self {
            config,
            buffer,
            writer: FrameBufferWriter::new(config),
        }
    }

    pub fn writer(&self) -> &FrameBufferWriter {
        &self.writer
    }

    pub fn copy(&self, pos: Vector2D<i32>, src: &FrameBuffer) {
        assert!(self.config.pixel_format == src.config.pixel_format);

        let dst_width = self.config.horizontal_resolution as i32;
        let dst_height = self.config.vertical_resolution as i32;
        let src_width = src.config.horizontal_resolution as i32;
        let src_height = src.config.vertical_resolution as i32;

        let copy_start_dst_x = max(pos.x, 0);
        let copy_start_dst_y = max(pos.y, 0);
        let copy_end_dst_x = min(pos.x + src_width, dst_width);
        let copy_end_dst_y = min(pos.y + src_height, dst_height);

        let bytes_per_pixel = self.config.pixel_format.bytes_per_pixel();
        let bytes_per_copy_line = bytes_per_pixel * (copy_end_dst_x - copy_start_dst_x) as usize;

        let pixels_per_scan_line = self.config.pixels_per_scan_line as usize;
        let i = bytes_per_pixel
            * (pixels_per_scan_line * copy_start_dst_y as usize + copy_start_dst_x as usize);

        let mut dst_buf = unsafe { self.config.frame_buffer.offset(i as isize) };
        let mut src_buf = src.config.frame_buffer;

        for _ in 0..copy_end_dst_y - copy_start_dst_y {
            unsafe {
                copy_nonoverlapping(src_buf, dst_buf, bytes_per_copy_line);
                dst_buf = dst_buf.offset((bytes_per_pixel * pixels_per_scan_line) as isize);
                src_buf = src_buf
                    .offset((bytes_per_pixel * src.config.pixels_per_scan_line as usize) as isize);
            }
        }
    }
}
