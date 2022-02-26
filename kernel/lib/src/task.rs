pub mod global {
    use crate::asm::{get_cr3, switch_context};
    use crate::segment::{KERNEL_CS, KERNEL_SS};
    use crate::task::TaskContext;
    use crate::timer::global::timer_manager;
    use crate::timer::{Timer, TASK_TIMER_PERIOD, TASK_TIMER_VALUE};
    use core::arch::asm;
    use core::ptr;

    static mut TASK_A_CTX: TaskContext = TaskContext::default_();
    fn task_a_ctx() -> &'static mut TaskContext {
        unsafe { &mut TASK_A_CTX }
    }

    static mut TASK_B_CTX: TaskContext = TaskContext::default_();
    pub fn task_b_ctx() -> &'static mut TaskContext {
        unsafe { &mut TASK_B_CTX }
    }

    static mut CURRENT_TASK: &TaskContext = unsafe { &TASK_A_CTX };

    pub fn initialize() {
        unsafe { asm!("cli") };
        timer_manager().add_timer(Timer::new(
            timer_manager().current_tick() + TASK_TIMER_PERIOD,
            TASK_TIMER_VALUE,
        ));
        unsafe { asm!("sti") };
    }

    pub fn initialize_task_b(rip: usize, task_b_stack_end: u64) {
        task_b_ctx().rip = rip as u64;
        task_b_ctx().rdi = 1;
        task_b_ctx().rsi = 43;

        task_b_ctx().cr3 = get_cr3();
        task_b_ctx().rflags = 0x202;
        task_b_ctx().cs = KERNEL_CS as u64;
        task_b_ctx().ss = KERNEL_SS as u64;
        task_b_ctx().rsp = (task_b_stack_end & !0xf) - 8;
        task_b_ctx().fxsave_area[24..][..4].copy_from_slice(&0x1f80u32.to_le_bytes());
    }

    pub fn switch_task() {
        unsafe {
            let old_current_task = CURRENT_TASK;
            let current_task = if ptr::eq(old_current_task as *const _, task_a_ctx() as *const _) {
                &TASK_B_CTX
            } else {
                &TASK_A_CTX
            };
            CURRENT_TASK = current_task;
            switch_context(current_task, old_current_task);
        }
    }
}

#[derive(Debug)]
#[repr(C, align(16))]
pub struct TaskContext {
    // offset : 0x00
    cr3: u64,
    rip: u64,
    rflags: u64,
    reserved1: u64,
    // offset : 0x20
    cs: u64,
    ss: u64,
    fs: u64,
    gs: u64,
    // offset : 0x40
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rdi: u64,
    rsi: u64,
    rsp: u64,
    rbp: u64,
    // offset : 0x80
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    // offset : 0xc0
    fxsave_area: [u8; 512],
}

impl TaskContext {
    const fn default_() -> Self {
        Self {
            cr3: 0,
            rip: 0,
            rflags: 0,
            reserved1: 0,
            cs: 0,
            ss: 0,
            fs: 0,
            gs: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rdi: 0,
            rsi: 0,
            rsp: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            fxsave_area: [0; 512],
        }
    }
}
