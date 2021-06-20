extern "C" {
    fn IoOut32(addr: u16, data: u32);
    fn IoIn32(addr: u16) -> u32;
    fn GetCS() -> u16;
    fn LoadIDT(limit: u16, offset: u64);
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
