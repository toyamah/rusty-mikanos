#[repr(C)]
pub struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

pub fn notify_end_of_interrupt() {
    let end_of_interrupt = 0xfee000b0 as *mut u32;
    unsafe {
        end_of_interrupt.write_volatile(0);
    }
}
