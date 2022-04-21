pub mod global {
    use crate::rust_official::cchar::c_char;
    use core::ffi::c_void;

    extern "C" {
        fn IoOut32(addr: u16, data: u32);
        fn IoIn32(addr: u16) -> u32;
        fn GetCS() -> u16;
        fn LoadIDT(limit: u16, offset: u64);
        fn LoadGDT(limit: u16, offset: u64);
        fn SetDSAll(value: u16);
        fn SetCSSS(cs: u16, ss: u16);
        fn SetCR3(value: u64);
        fn GetCR3() -> u64;
        fn SwitchContext(next_ctx: *const c_void, current_ctx: *const c_void);
        fn CallApp(argc: i32, argv: *const *const c_char, cs: u16, ss: u16, rip: u64, rsp: u64);
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

    pub fn set_ds_all(value: u16) {
        unsafe { SetDSAll(value) }
    }

    pub fn set_csss(cs: u16, ss: u16) {
        unsafe { SetCSSS(cs, ss) }
    }

    pub fn set_cr3(value: u64) {
        unsafe { SetCR3(value) }
    }

    pub fn get_cr3() -> u64 {
        unsafe { GetCR3() }
    }

    /// # Safety
    pub unsafe fn switch_context<T>(next_ctx: &T, current_ctx: &T) {
        SwitchContext(
            next_ctx as *const _ as *const c_void,
            current_ctx as *const _ as *const c_void,
        );
    }

    /// # Safety
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn call_app(argc: i32, argv: *const *const c_char, cs: u16, ss: u16, rip: u64, rsp: u64) {
        unsafe { CallApp(argc, argv, cs, ss, rip, rsp) }
    }
}
