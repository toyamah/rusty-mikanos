use crate::c_char;
use crate::syscall::{SyscallOpenFile, SyscallPutString, SyscallReadFile};
use core::ffi::c_void;

static mut ERRNO: i32 = 0;

pub enum FILE {}

#[no_mangle]
pub extern "C" fn close(_fd: i32) -> i32 {
    unsafe { ERRNO = 9 }; // EBADF
    -1
}

#[no_mangle]
pub extern "C" fn fstat(_fd: i32, _buf: *const c_void) -> i32 {
    unsafe { ERRNO = 9 }; // EBADF
    -1
}

#[no_mangle]
pub extern "C" fn getpid() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn isatty(_fd: i32) -> i32 {
    unsafe { ERRNO = 9 }; // EBADF
    -1
}

#[no_mangle]
pub extern "C" fn lseek(_fd: i32, _offset: i64, _whence: i32) -> i32 {
    unsafe { ERRNO = 9 }; // EBADF
    -1
}

#[no_mangle]
pub extern "C" fn open(path: *const c_char, flags: i32) -> i32 {
    let res = unsafe { SyscallOpenFile(path, flags) };
    if res.is_ok() {
        res.value as i32
    } else {
        unsafe { ERRNO = res.error };
        -1
    }
}

#[no_mangle]
pub extern "C" fn read(fd: i32, buf: *const c_void, count: usize) -> i32 {
    let res = unsafe { SyscallReadFile(fd, buf, count) };
    if res.is_ok() {
        res.value as i32
    } else {
        unsafe { ERRNO = res.error };
        -1
    }
}

static mut HEAP: [u8; 4096] = [0; 4096];
static mut I: i32 = 0;

#[no_mangle]
pub extern "C" fn sbrk(incr: i32) -> *const c_void {
    let prev = unsafe { I };
    unsafe { I += incr };

    unsafe { &HEAP[prev as usize] as *const _ as *const c_void }
}

#[no_mangle]
pub extern "C" fn write(fd: i32, buf: *const c_void, count: usize) -> isize {
    let res = unsafe { SyscallPutString(fd, buf, count) };

    if res.is_ok() {
        res.value as isize
    } else {
        unsafe { ERRNO = res.error };
        -1
    }
}

pub fn exit(status: i32) -> ! {
    unsafe { crate::libc::exit(status) }
}
