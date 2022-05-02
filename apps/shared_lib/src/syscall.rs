use crate::c_char;

extern "C" {
    pub(crate) fn SyscallPutString(fd: i32, buf: usize, count: usize) -> SyscallResult;
    pub(crate) fn SyscallExit(exit_code: i32);
    pub(crate) fn SyscallLogString(level: i64, s: *const c_char) -> SyscallResult;
}

#[repr(C)]
pub(crate) struct SyscallResult {
    pub(crate) value: u64,
    pub(crate) error: i32,
}

impl SyscallResult {
    pub(crate) fn is_ok(&self) -> bool {
        self.error == 0
    }
}
