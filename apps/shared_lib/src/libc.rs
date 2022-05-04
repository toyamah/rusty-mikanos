use crate::c_char;
use crate::syscall::SyscallExit;

extern "C" {
    pub fn strlen(cs: *const c_char) -> usize;
    pub fn atol(s: *const c_char) -> i64;
    // pub fn strcmp(a: *const c_char, b: *const c_char) -> i32;
    pub(crate) fn exit(status: i32) -> !;
    pub(crate) fn strerror(n: i32) -> *mut c_char;
}

#[no_mangle]
pub extern "C" fn _exit(status: i32) {
    unsafe { SyscallExit(status) }
}
