use crate::asm::load_interrupt_descriptor_table;
use crate::segment::KERNEL_CS;
use crate::x86_descriptor::SystemDescriptorType;
use bit_field::BitField;

// IDT can have 256(0-255) descriptors
static mut IDT: [InterruptDescriptor; 256] = [InterruptDescriptor {
    offset_low: 0,
    segment_selector: 0,
    attr: InterruptDescriptorAttribute(0),
    offset_middle: 0,
    offset_high: 0,
    reserved: 0,
}; 256];
/// idt stands for Interrupt Descriptor Table which maps interruption vector numbers to interrupt handlers.
/// https://en.wikipedia.org/wiki/Interrupt_descriptor_table
pub fn idt() -> &'static mut [InterruptDescriptor; 256] {
    unsafe { &mut IDT }
}

pub fn notify_end_of_interrupt() {
    //  Writing to this register means the CPU can know an interrupt routine has been completed.
    let end_of_interrupt_register = 0xfee000b0 as *mut u32;
    unsafe {
        // Use write_volatile to ignore compiler optimization.
        end_of_interrupt_register.write_volatile(0);
    }
}

pub fn initialize_interrupt(offset: usize) {
    let idt = idt();
    idt[InterruptVectorNumber::XHCI as usize].set_idt_entry(
        InterruptDescriptorAttribute::new(SystemDescriptorType::InterruptGate, 0, true, 0),
        offset as u64,
        KERNEL_CS,
    );
    load_interrupt_descriptor_table(
        core::mem::size_of_val(idt) as u16,
        &idt[0] as *const InterruptDescriptor as u64,
    );
}

#[repr(C)]
pub struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl InterruptDescriptor {
    pub fn set_idt_entry(
        &mut self,
        attr: InterruptDescriptorAttribute,
        offset: u64,
        segment_selector: u16,
    ) {
        self.attr = attr;
        self.offset_low = offset.get_bits(0..16) as u16;
        self.offset_middle = offset.get_bits(16..32) as u16;
        self.offset_high = offset.get_bits(32..) as u32;
        self.segment_selector = segment_selector;
    }
}

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct InterruptDescriptorAttribute(u16);

impl InterruptDescriptorAttribute {
    pub fn new(
        descriptor_type: SystemDescriptorType,
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

pub enum InterruptVectorNumber {
    XHCI = 0x40,
}
