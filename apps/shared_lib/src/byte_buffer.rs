use crate::c_char;
use core::ffi::c_void;
use core::fmt;
use core::fmt::Write;

/// Buffer to avoid memory allocation
pub(crate) struct ByteBuffer {
    next: usize,
    buf: [u8; 1024],
}

impl ByteBuffer {
    pub(crate) fn new() -> Self {
        Self {
            next: 0,
            buf: [0; 1024],
        }
    }

    pub(crate) fn as_ptr(&self) -> *const u8 {
        &self.buf as *const _
    }

    pub(crate) fn as_ptr_void(&self) -> *const c_void {
        &self.buf as *const _ as *const c_void
    }

    pub(crate) fn as_ptr_c_char(&self) -> *const c_char {
        &self.buf as *const _ as *const c_char
    }

    pub(crate) fn write_str_with_nul(&mut self, s: &str) {
        self.write_str(s).unwrap();
        self.write_str("\0").unwrap();
    }

    pub(crate) fn len(&self) -> usize {
        self.next
    }
}

impl Write for ByteBuffer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        if self.buf.len() < self.next + bytes.len() {
            return Result::Err(fmt::Error::default());
        }

        for &b in bytes {
            self.buf[self.next] = b;
            self.next += 1;
        }
        Result::Ok(())
    }
}
