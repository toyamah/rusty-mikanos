use crate::syscall::SyscallExit;

extern "C" {
    pub(crate) fn exit(status: i32) -> !;
}

#[no_mangle]
pub extern "C" fn _exit(status: i32) {
    unsafe { SyscallExit(status) }
}
