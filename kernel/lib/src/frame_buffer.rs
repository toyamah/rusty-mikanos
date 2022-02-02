use crate::graphics::{FrameBufferWriter, Vector2D};
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::copy_nonoverlapping;
use shared::FrameBufferConfig;

pub struct FrameBuffer {
    config: FrameBufferConfig,
    // unused. handle it through config.frame_buffer instead.
    _buffer: Vec<u8>,
    writer: FrameBufferWriter,
}

impl FrameBuffer {
    pub fn new(mut config: FrameBufferConfig) -> Self {
        let _buffer = if config.frame_buffer.is_null() {
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
            _buffer,
            writer: FrameBufferWriter::new(config),
        }
    }

    pub fn writer(&self) -> &FrameBufferWriter {
        &self.writer
    }

    pub fn copy(&self, pos: Vector2D<i32>, src: &FrameBuffer) {
        assert!(self.config.pixel_format == src.config.pixel_format);

        let dst_size = frame_buffer_size(&self.config);
        let src_size = frame_buffer_size(&src.config);
        let dst_start = pos.element_max(Vector2D::new(0, 0));
        let dst_end = dst_size.element_min(Vector2D::new(
            pos.x + src_size.x as i32,
            pos.y + src_size.y as i32,
        ));
        let mut dst_buf = unsafe {
            self.config
                .frame_addr_at(dst_start.x as usize, dst_start.y as usize)
        };
        let mut src_buf = unsafe { src.config.frame_addr_at(0, 0) };

        let bytes_per_copy_line =
            self.config.pixel_format.bytes_per_pixel() * (dst_end.x - dst_start.x) as usize;
        for _ in dst_start.y..dst_end.y {
            unsafe {
                copy_nonoverlapping(src_buf, dst_buf, bytes_per_copy_line);
                dst_buf = dst_buf.add(self.config.bytes_per_scan_line());
                src_buf = src_buf.add(src.config.bytes_per_scan_line());
            }
        }
    }
}

fn frame_buffer_size(config: &FrameBufferConfig) -> Vector2D<i32> {
    Vector2D::new(
        config.horizontal_resolution as i32,
        config.vertical_resolution as i32,
    )
}
