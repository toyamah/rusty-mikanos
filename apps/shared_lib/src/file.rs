use crate::libc::{fgets, fopen};
use crate::newlib_support::FILE;
use crate::rust_official::cstr::CStr;
use crate::{c_char, ByteBuffer};
use core::str::Utf8Error;

#[derive(Copy, Clone, Debug)]
pub enum OpenMode {
    R,
}

impl OpenMode {
    fn to_str(&self) -> &str {
        match self {
            OpenMode::R => "r",
        }
    }
}

pub fn open_file(path: &str, mode: OpenMode) -> *mut FILE {
    let mut buf = ByteBuffer::new();
    buf.write_str_with_nul(path);

    let p = buf.as_ptr_c_char();
    let m = mode.to_str().as_ptr() as *const c_char;
    unsafe { fopen(p, m) }
}

pub fn read_file(file: *mut FILE, buf: &mut [u8]) -> *mut c_char {
    let b = buf as *mut _ as *mut c_char;
    unsafe { fgets(b, buf.len() as i32, file) }
}

pub fn buf_to_str(buf: &[u8]) -> Result<&str, Utf8Error> {
    unsafe { CStr::from_ptr(buf as *const _ as *const c_char) }.to_str()
}
