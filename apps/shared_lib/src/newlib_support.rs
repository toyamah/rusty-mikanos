use core::ffi::c_void;

extern "C" {
    pub(crate) fn SyscallPutString(fd: i32, buf: usize, count: usize) -> SyscallResult;
    fn SyscallExit(exit_code: i32);
}

#[repr(C)]
pub(crate) struct SyscallResult {
    value: u64,
    error: i32,
}

impl SyscallResult {
    fn is_ok(&self) -> bool {
        self.error == 0
    }
}

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

pub fn exit(status: i32) {
    unsafe { SyscallExit(status) };
}
