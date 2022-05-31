use crate::c_char;
use crate::syscall::{SyscallDemandPages, SyscallOpenFile, SyscallPutString, SyscallReadFile};
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
static mut DPAGE_END: i64 = 0;
static mut PROGRAM_BREAK: i64 = 0;

#[no_mangle]
pub extern "C" fn sbrk(incr: i32) -> i64 {
    unsafe {
        let incr = incr as i64;
        if DPAGE_END == 0 || (DPAGE_END as i64) < PROGRAM_BREAK as i64 + incr {
            let num_pages = (incr + 4095) / 4096;
            let res = SyscallDemandPages(num_pages as usize, 0);
            if !res.is_ok() {
                ERRNO = res.error;
                // return -1 as *const _ as *const c_void;
                return -1;
            }

            PROGRAM_BREAK = res.value as i64;
            DPAGE_END = res.value as i64 + 4096 * num_pages;
        }

        let prev_break = PROGRAM_BREAK;
        PROGRAM_BREAK = PROGRAM_BREAK + incr;
        prev_break as i64
    }
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
