use crate::x86_descriptor::SegmentDescriptorType;
use bit_field::BitField;

pub const KERNEL_CS: u16 = 1 << 3;
pub const KERNEL_SS: u16 = 2 << 3;
const KERNEL_DS: u16 = 0;
const K_TSS: u16 = 5 << 3;

pub mod global {
    use super::{SegmentDescriptor, KERNEL_CS, KERNEL_DS, KERNEL_SS};
    use crate::asm::global::{load_gdt, load_tr, set_csss, set_ds_all};
    use crate::interrupt::global::IST_FOR_TIMER;
    use crate::memory_manager::global::MEMORY_MANAGER;
    use crate::memory_manager::BYTES_PER_FRAME;
    use crate::segment::K_TSS;
    use crate::x86_descriptor::SegmentDescriptorType;
    use core::arch::asm;
    use core::mem;

    static mut GDT: [SegmentDescriptor; 7] = [SegmentDescriptor::new(); 7];
    static mut TSS: [u32; 26] = [0; 26];

    pub fn initialize() {
        set_up_segment();
        set_ds_all(KERNEL_DS);
        set_csss(KERNEL_CS, KERNEL_SS);
    }

    fn set_up_segment() {
        unsafe {
            asm!("cli");
            GDT[1].set_code_segment(SegmentDescriptorType::ExecuteRead, 0, 0, 0xfffff);
            GDT[2].set_data_segment(SegmentDescriptorType::ReadWrite, 0, 0, 0xfffff);
            GDT[3].set_data_segment(SegmentDescriptorType::ReadWrite, 3, 0, 0xfffff);
            GDT[4].set_code_segment(SegmentDescriptorType::ExecuteRead, 3, 0, 0xfffff);
            load_gdt(
                core::mem::size_of_val(&GDT) as u16 - 1,
                &GDT[0] as *const _ as u64,
            );
            asm!("sti");
        }
    }

    pub fn initialize_tss() {
        unsafe { asm!("cli") };
        set_tss(1, allocate_stack_area(8));
        set_tss(7 + 2 * IST_FOR_TIMER as usize, allocate_stack_area(8));

        let tss_addr = unsafe { &TSS[0] as *const _ as usize };
        unsafe {
            let i = (K_TSS >> 3) as usize;
            GDT[i].set_system_segment(
                SegmentDescriptorType::TSSAvailable,
                0,
                (tss_addr & 0xffff_ffff) as u32,
                (mem::size_of_val(&TSS) - 1) as u32,
            );
            GDT[i + 1] = SegmentDescriptor((tss_addr >> 32) as u64);
        }
        load_tr(K_TSS);
        unsafe { asm!("sti") };
    }

    fn set_tss(index: usize, value: u64) {
        unsafe {
            TSS[index] = (value & 0xffffffff) as u32;
            TSS[index + 1] = (value >> 32) as u32;
        }
    }

    fn allocate_stack_area(num_4kframes: usize) -> u64 {
        if let Ok(frame_id) = MEMORY_MANAGER.lock().allocate(num_4kframes) {
            (frame_id.id() * BYTES_PER_FRAME + num_4kframes * 4096) as u64
        } else {
            panic!("failed to allocate stack area. num = {}", num_4kframes);
        }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct SegmentDescriptor(u64);

impl SegmentDescriptor {
    const fn new() -> Self {
        Self(0)
    }

    fn set_code_segment(
        &mut self,
        type_: SegmentDescriptorType,
        descriptor_privilege_level: u32,
        base: u32,
        limit: u32,
    ) {
        let base = base as u64;
        self.set_base_low(base & 0xffff);
        self.set_base_middle((base >> 16) & 0xff);
        self.set_base_high((base >> 24) & 0xff);

        let limit = limit as u64;
        self.set_limit_low(limit & 0xffff);
        self.set_limit_high((limit >> 16) & 0xf);

        self.set_type(type_);
        self._set_system_segment(1); // 1: code & data segment
        self.set_descriptor_privilege_level(descriptor_privilege_level as u64);
        self.set_present(1);
        self.set_available(0);
        self.set_long_mode(1);
        self.set_default_operation_size(0); // should be 0 when long mode == 1
        self.set_granularity(1);
    }

    pub fn set_data_segment(
        &mut self,
        type_: SegmentDescriptorType,
        descriptor_privilege_level: u32,
        base: u32,
        limit: u32,
    ) {
        self.set_code_segment(type_, descriptor_privilege_level, base, limit);
        self.set_long_mode(0);
        self.set_default_operation_size(1); // 32-bit stack segment
    }

    pub fn set_system_segment(
        &mut self,
        type_: SegmentDescriptorType,
        descriptor_privilege_level: u32,
        base: u32,
        limit: u32,
    ) {
        self.set_code_segment(type_, descriptor_privilege_level, base, limit);
        self._set_system_segment(0);
        self.set_long_mode(0);
    }

    // uint64_t limit_low : 16; 0..16
    fn set_limit_low(&mut self, v: u64) {
        self.0.set_bits(0..16, v);
    }

    // uint64_t base_low : 16; 16..32
    fn set_base_low(&mut self, v: u64) {
        self.0.set_bits(16..32, v);
    }
    // uint64_t base_middle : 8; 32..40
    fn set_base_middle(&mut self, v: u64) {
        self.0.set_bits(32..40, v);
    }

    // DescriptorType type : 4; 40..44
    fn set_type(&mut self, t: SegmentDescriptorType) {
        self.0.set_bits(40..44, t as u64);
    }

    // uint64_t system_segment : 1; 44..45
    fn _set_system_segment(&mut self, v: u64) {
        self.0.set_bits(44..45, v);
    }

    // uint64_t descriptor_privilege_level : 2; 45..47
    fn set_descriptor_privilege_level(&mut self, v: u64) {
        self.0.set_bits(45..47, v);
    }

    // uint64_t present : 1; 47..48
    fn set_present(&mut self, v: u64) {
        self.0.set_bits(47..48, v);
    }

    // uint64_t limit_high : 4; 48..52
    fn set_limit_high(&mut self, v: u64) {
        self.0.set_bits(48..52, v);
    }

    // uint64_t available : 1; 52..53
    fn set_available(&mut self, v: u64) {
        self.0.set_bits(52..53, v);
    }

    // uint64_t long_mode : 1; 53..54
    fn set_long_mode(&mut self, v: u64) {
        self.0.set_bits(53..54, v);
    }

    // uint64_t default_operation_size : 1; 54..55
    fn set_default_operation_size(&mut self, v: u64) {
        self.0.set_bits(54..55, v);
    }

    // uint64_t granularity : 1; 55 .. 56
    fn set_granularity(&mut self, v: u64) {
        self.0.set_bits(55..56, v);
    }

    // uint64_t base_high : 8; 56..64
    fn set_base_high(&mut self, v: u64) {
        self.0.set_bits(56..64, v);
    }
}
