use crate::c_char;

extern "C" {
    pub(crate) fn SyscallPutString(fd: i32, buf: usize, count: usize) -> SyscallResult;
    pub(crate) fn SyscallExit(exit_code: i32) -> !;
    pub(crate) fn SyscallLogString(level: i64, s: *const c_char) -> SyscallResult;

    pub(crate) fn SyscallOpenWindow(
        w: i32,
        h: i32,
        x: i32,
        y: i32,
        title: *const c_char,
    ) -> SyscallResult;

    pub(crate) fn SyscallWinWriteString(
        layer_id: u32,
        x: i32,
        y: i32,
        color: u32,
        s: *const c_char,
    ) -> SyscallResult;
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

    pub fn to_result(&self) -> Result<u64, SyscallError> {
        if self.is_ok() {
            Ok(self.value)
        } else {
            Err(SyscallError::new(self.value, self.error))
        }
    }
}

pub struct SyscallError {
    value: u64,
    error_number: i32,
}

impl SyscallError {
    pub fn new(value: u64, error_number: i32) -> Self {
        Self {
            value,
            error_number,
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn error_number(&self) -> i32 {
        self.error_number
    }
}
