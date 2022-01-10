extern "C" {
    fn IoOut32(addr: u16, data: u32);
    fn IoIn32(addr: u16) -> u32;
    fn GetCS() -> u16;
    fn LoadIDT(limit: u16, offset: u64);
    fn LoadGDT(limit: u16, offset: u64);
    fn SetDSAll(value: u16);
    fn SetCSSS(cs: u16, ss: u16);
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
