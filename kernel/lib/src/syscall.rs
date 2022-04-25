use crate::asm::global::{write_msr, SyscallEntry};
use crate::msr::{IA32_EFFR, IA32_FMASK, IA32_LSTAR, IA32_STAR};
use crate::rust_official::c_str::CStr;
use crate::rust_official::cchar::c_char;
use alloc::string::String;
use log::{log, Level};

type SyscallFuncType = fn(u64, u64, u64, u64, u64, u64) -> i64;

fn log_string(a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64) -> i64 {
    let log_level = match a1 {
        1 => Level::Error,
        2 => Level::Warn,
        3 => Level::Info,
        4 => Level::Debug,
        5 => Level::Trace,
        _ => return -1,
    };

    let c_str = unsafe { CStr::from_ptr(a2 as *const u64 as *const c_char) };
    log!(log_level, "{}\n", String::from_utf8_lossy(c_str.to_bytes()));
    0
}

#[no_mangle]
static mut syscall_table: [SyscallFuncType; 1] = [log_string];

pub fn initialize_syscall() {
    write_msr(IA32_EFFR, 0x0501);
    write_msr(IA32_LSTAR, SyscallEntry as usize as u64);
    write_msr(IA32_STAR, 8 << 32 | (16 | 3) << 48);
    write_msr(IA32_FMASK, 0);
}
