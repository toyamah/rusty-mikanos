use bit_field::BitField;

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

#[repr(C)]
pub struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct InterruptDescriptorAttribute(u16);

impl InterruptDescriptorAttribute {
    pub fn new(
        descriptor_type: DescriptorType,
        descriptor_privilege_level: u8,
        present: bool,
        interrupt_stack_table: u8,
    ) -> Self {
        let mut field: u16 = 0;
        field
            .set_bit(15, present)
            .set_bits(13..15, descriptor_privilege_level as u16)
            .set_bits(8..12, descriptor_type as u16)
            .set_bits(0..3, interrupt_stack_table as u16);
        Self(field)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Upper8Bytes = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}
