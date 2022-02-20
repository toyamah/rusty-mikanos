use crate::graphics::{FrameBufferWriter, Rectangle, Vector2D};
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

    pub fn writer(&mut self) -> &mut FrameBufferWriter {
        &mut self.writer
    }

    pub fn config(&self) -> &FrameBufferConfig {
        &self.config
    }

    pub fn copy(&self, dst_pos: Vector2D<i32>, src: &FrameBuffer, src_area: Rectangle<i32>) {
        assert!(self.config.pixel_format == src.config.pixel_format);

        let src_area_shifted = Rectangle::new(dst_pos, src_area.size);
        let src_outline = Rectangle::new(dst_pos - src_area.pos, frame_buffer_size(&self.config));
        let dst_outline = Rectangle::new(Vector2D::new(0, 0), frame_buffer_size(&self.config));
        let copy_area = dst_outline & src_outline & src_area_shifted;
        let src_start_pos = copy_area.pos - (dst_pos - src_area.pos);

        let mut dst_buf = unsafe { self.config.frame_addr_at(copy_area.pos.x, copy_area.pos.y) };
        let mut src_buf = unsafe { src.config.frame_addr_at(src_start_pos.x, src_start_pos.y) };

        let bytes_per_copy_line =
            self.config.pixel_format.bytes_per_pixel() * copy_area.size.x as usize;
        for _ in 0..copy_area.size.y {
            unsafe {
                copy_nonoverlapping(src_buf, dst_buf, bytes_per_copy_line);
                dst_buf = dst_buf.add(self.config.bytes_per_scan_line());
                src_buf = src_buf.add(src.config.bytes_per_scan_line());
            }
        }
    }

    pub fn move_(&mut self, dst_pos: Vector2D<i32>, src: &Rectangle<i32>) {
        let bytes_per_pixel = self.config.pixel_format.bytes_per_pixel();
        let bytes_per_scan_line = self.config.bytes_per_scan_line();

        if dst_pos.y < src.pos.y {
            // move up
            let mut dst_buf = unsafe { self.config.frame_addr_at(dst_pos.x, dst_pos.y) };
            let mut src_buf = unsafe { self.config.frame_addr_at(src.pos.x, src.pos.y) };
            for _ in 0..src.size.y {
                unsafe {
                    copy_nonoverlapping(src_buf, dst_buf, bytes_per_pixel * src.size.x as usize);
                    dst_buf = dst_buf.add(bytes_per_scan_line);
                    src_buf = src_buf.add(bytes_per_scan_line);
                }
            }
        } else {
            // // move down
            let mut dst_buf = unsafe {
                self.config
                    .frame_addr_at(dst_pos.x, dst_pos.y + src.size.y - 1)
            };
            let mut src_buf = unsafe {
                self.config
                    .frame_addr_at(src.pos.x, src.pos.y + src.size.y - 1)
            };
            for _ in 0..src.size.y {
                unsafe {
                    copy_nonoverlapping(src_buf, dst_buf, bytes_per_pixel * src.size.x as usize);
                    dst_buf = dst_buf.sub(bytes_per_scan_line);
                    src_buf = src_buf.sub(bytes_per_scan_line);
                }
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
