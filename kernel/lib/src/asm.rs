pub mod global {
    use crate::rust_official::cchar::c_char;
    use crate::task::TaskContext;
    use core::ffi::c_void;

    extern "C" {
        fn IoOut32(addr: u16, data: u32);
        fn IoIn32(addr: u16) -> u32;
        fn GetCS() -> u16;
        fn LoadIDT(limit: u16, offset: u64);
        fn LoadGDT(limit: u16, offset: u64);
        fn LoadTR(sel: u16);
        fn SetDSAll(value: u16);
        fn SetCSSS(cs: u16, ss: u16);
        fn GetCR0() -> u64;
        fn SetCR0(value: u64) -> u64;
        fn GetCR2() -> u64;
        fn SetCR3(value: u64);
        fn GetCR3() -> u64;
        fn SwitchContext(next_ctx: *const c_void, current_ctx: *const c_void);
        fn RestoreContext(task_context: *const c_void);
        pub fn CallApp(
            argc: i32,
            argv: *const *const c_char,
            ss: u16,
            rip: u64,
            rsp: u64,
            os_stack_ptr: *const u64,
        ) -> i32;
        pub fn IntHandlerLAPICTimer();
        fn WriteMSR(msr: u32, value: u64);
        pub fn SyscallEntry();
        fn ExitApp(rsp: u64, ret_val: i32);
        fn InvalidateTLB(addr: u64);
    }

    pub fn io_out_32(addr: u16, data: u32) {
        unsafe { IoOut32(addr, data) };
    }

    pub fn io_in_32(addr: u16) -> u32 {
        unsafe { IoIn32(addr) }
    }

    pub fn get_code_segment() -> u16 {
        unsafe { GetCS() }
    }

    pub fn load_interrupt_descriptor_table(limit: u16, offset: u64) {
        unsafe { LoadIDT(limit, offset) }
    }

    pub fn load_gdt(limit: u16, offset: u64) {
        unsafe { LoadGDT(limit, offset) }
    }

    pub fn load_tr(sel: u16) {
        unsafe { LoadTR(sel) }
    }

    pub fn set_ds_all(value: u16) {
        unsafe { SetDSAll(value) }
    }

    pub fn set_csss(cs: u16, ss: u16) {
        unsafe { SetCSSS(cs, ss) }
    }

    pub fn get_cr0() -> u64 {
        unsafe { GetCR3() }
    }

    pub fn set_cr0(value: u64) {
        unsafe { SetCR3(value) }
    }

    pub fn get_cr2() -> u64 {
        unsafe { GetCR2() }
    }

    pub fn set_cr3(value: u64) {
        unsafe { SetCR3(value) }
    }

    pub fn get_cr3() -> u64 {
        unsafe { GetCR3() }
    }

    /// # Safety
    pub unsafe fn switch_context(next_ctx: &TaskContext, current_ctx: &TaskContext) {
        SwitchContext(
            next_ctx as *const _ as *const c_void,
            current_ctx as *const _ as *const c_void,
        );
    }

    /// # Safety
    pub unsafe fn restore_context(task_context: &TaskContext) {
        RestoreContext(task_context as *const _ as *const c_void);
    }

    /// # Safety
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn call_app(
        argc: i32,
        argv: *const *const c_char,
        ss: u16,
        rip: u64,
        rsp: u64,
        os_stack_ptr: *const u64,
    ) -> i32 {
        unsafe { CallApp(argc, argv, ss, rip, rsp, os_stack_ptr) }
    }

    pub fn write_msr(msr: u32, value: u64) {
        unsafe { WriteMSR(msr, value) }
    }

    pub fn exit_app(rsp: u64, ret_val: i32) {
        unsafe { ExitApp(rsp, ret_val) }
    }

    pub fn invalidate_tlb(addr: u64) {
        unsafe { InvalidateTLB(addr) }
    }
}
