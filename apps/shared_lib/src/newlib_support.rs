use crate::syscall::{SyscallExit, SyscallPutString};
use core::ffi::c_void;

static mut ERRNO: i32 = 0;

pub fn write(fd: i32, buf: *const c_void, count: usize) -> isize {
    let res = unsafe { SyscallPutString(fd, buf as usize, count) };

    if res.is_ok() {
        res.value as isize
    } else {
        unsafe { ERRNO = res.error };
        -1
    }
}

pub fn exit(status: i32) -> ! {
    unsafe { SyscallExit(status) }
}
