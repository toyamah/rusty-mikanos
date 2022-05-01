use crate::asm::global::{write_msr, SyscallEntry};
use crate::msr::{IA32_EFFR, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use crate::rust_official::c_str::CStr;
use crate::rust_official::cchar::c_char;
use log::{log, Level};

type SyscallFuncType = fn(u64, u64, u64, u64, u64, u64) -> CallResult;

// Execute command `errno -l` to see each error number
const EPERM: i32 = 1; // Operation not permitted (POSIX.1-2001).
const E2BIG: i32 = 7; // Argument list too long
const EBADF: i32 = 9; // Bad file descriptor

struct CallResult {
    value: u64,
    error: i32,
}

impl CallResult {
    fn err(value: u64, error: i32) -> Self {
        Self { value, error }
    }

    fn ok(value: u64) -> Self {
        Self { value, error: 0 }
    }
}

fn log_string(log_level: u64, s: u64, _a3: u64, _a4: u64, _a5: u64, _a6: u64) -> CallResult {
    let log_level = match log_level {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        5 => Level::Trace,
        _ => return CallResult::err(0, EPERM),
    };

    let c_str = unsafe { c_str_from(s) };
    let len = c_str.to_bytes().len();
    if len > 1024 {
        return CallResult::err(0, E2BIG);
    }
    let str = str_from(c_str.to_bytes());
    log!(log_level, "{}", str);
    CallResult::ok(len as u64)
}

unsafe fn c_str_from<'a>(p: u64) -> &'a CStr {
    CStr::from_ptr(p as *const u64 as *const c_char)
}

fn str_from(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).expect("could not convert to str")
}

#[no_mangle]
static mut syscall_table: [SyscallFuncType; 1] = [log_string];

pub fn initialize_syscall() {
    write_msr(IA32_EFFR, 0x0501);
    write_msr(IA32_LSTAR, SyscallEntry as usize as u64);
    write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    write_msr(IA32_FMASK, 0);
}
