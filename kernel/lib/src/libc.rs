use crate::rust_official::cchar::c_char;
use core::ffi::c_void;

extern "C" {
    pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void;
    pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}
