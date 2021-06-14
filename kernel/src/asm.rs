extern "C" {
    //TODO: move other functions here
    fn GetCS() -> u16;
    fn LoadIDT(limit: u16, offset: u64);
}

pub fn get_code_segment() -> u16 {
    unsafe { GetCS() }
}

pub fn load_interrupt_descriptor_table(limit: u16, offset: u64) {
    unsafe { LoadIDT(limit, offset) }
}
