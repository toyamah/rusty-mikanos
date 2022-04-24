use crate::segment::KERNEL_CS;
use crate::x86_descriptor::SystemDescriptorType;
use bit_field::BitField;

pub mod global {
    use super::{InterruptDescriptor, InterruptDescriptorAttribute, InterruptVectorNumber};
    use crate::asm::global::{load_interrupt_descriptor_table, IntHandlerLAPICTimer};
    use crate::graphics::global::pixel_writer;
    use crate::graphics::{PixelWriter, COLOR_WHITE};
    use crate::interrupt::InterruptFrame;
    use crate::message::{Message, MessageType};
    use crate::task::global::task_manager;
    use core::arch::asm;

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

    pub fn initialize_interrupt() {
        let idt = idt();
        idt[InterruptVectorNumber::XHCI as usize].set_idt_entry(int_handler_xhci as usize);
        idt[InterruptVectorNumber::LAPICTimer as usize]
            .set_idt_entry(IntHandlerLAPICTimer as usize);
        idt[0].set_idt_entry(int_handler_de as usize);
        idt[1].set_idt_entry(int_handler_db as usize);
        idt[3].set_idt_entry(int_handler_bp as usize);
        idt[4].set_idt_entry(int_handler_of as usize);
        idt[5].set_idt_entry(int_handler_br as usize);
        idt[6].set_idt_entry(int_handler_ud as usize);
        idt[7].set_idt_entry(int_handler_nm as usize);
        idt[8].set_idt_entry(int_handler_df as usize);
        idt[10].set_idt_entry(int_handler_ts as usize);
        idt[11].set_idt_entry(int_handler_np as usize);
        idt[12].set_idt_entry(int_handler_ss as usize);
        idt[13].set_idt_entry(int_handler_gp as usize);
        idt[14].set_idt_entry(int_handler_pf as usize);
        idt[16].set_idt_entry(int_handler_mf as usize);
        idt[17].set_idt_entry(int_handler_ac as usize);
        idt[18].set_idt_entry(int_handler_mc as usize);
        idt[19].set_idt_entry(int_handler_xm as usize);
        idt[20].set_idt_entry(int_handler_ve as usize);

        load_interrupt_descriptor_table(
            core::mem::size_of_val(idt) as u16,
            &idt[0] as *const InterruptDescriptor as u64,
        );
    }

    extern "x86-interrupt" fn int_handler_xhci(_: *const InterruptFrame) {
        task_manager()
            .send_message(
                task_manager().main_task().id(),
                Message::new(MessageType::InterruptXhci),
            )
            .unwrap();
        notify_end_of_interrupt();
    }

    extern "x86-interrupt" fn int_handler_de(frame: *const InterruptFrame) {
        _fault_handler_no_error("#DE", frame);
    }
    extern "x86-interrupt" fn int_handler_db(frame: *const InterruptFrame) {
        _fault_handler_no_error("#db", frame);
    }
    extern "x86-interrupt" fn int_handler_bp(frame: *const InterruptFrame) {
        _fault_handler_no_error("#BP", frame);
    }
    extern "x86-interrupt" fn int_handler_of(frame: *const InterruptFrame) {
        _fault_handler_no_error("#OF", frame);
    }
    extern "x86-interrupt" fn int_handler_br(frame: *const InterruptFrame) {
        _fault_handler_no_error("#BR", frame);
    }
    extern "x86-interrupt" fn int_handler_ud(frame: *const InterruptFrame) {
        _fault_handler_no_error("#UD", frame);
    }
    extern "x86-interrupt" fn int_handler_nm(frame: *const InterruptFrame) {
        _fault_handler_no_error("#NM", frame);
    }
    extern "x86-interrupt" fn int_handler_df(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#DF", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_ts(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#TS", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_np(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#NP", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_ss(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#SS", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_gp(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#GP", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_pf(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#PF", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_mf(frame: *const InterruptFrame) {
        _fault_handler_no_error("#MF", frame);
    }
    extern "x86-interrupt" fn int_handler_ac(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#AC", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_mc(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#MC", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_xm(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#XM", frame, error_code);
    }
    extern "x86-interrupt" fn int_handler_ve(frame: *const InterruptFrame, error_code: u64) {
        _fault_handler_with_error("#VE", frame, error_code);
    }

    fn _fault_handler_with_error(name: &str, frame: *const InterruptFrame, error_code: u64) {
        let f = unsafe { frame.as_ref() }.unwrap();
        print_frame(f, name);
        pixel_writer().write_string(500, 16 * 4, "ERR", &COLOR_WHITE);
        print_hex(error_code, 16, 500 + 8 * 4, 16 * 4);
        loop {
            unsafe { asm!("hlt") }
        }
    }

    fn _fault_handler_no_error(name: &str, frame: *const InterruptFrame) {
        let f = unsafe { frame.as_ref() }.unwrap();
        print_frame(f, name);
        loop {
            unsafe { asm!("hlt") }
        }
    }

    pub fn print_frame(f: &InterruptFrame, exp_name: &str) {
        let w = pixel_writer();
        w.write_string(500, 0, exp_name, &COLOR_WHITE);

        w.write_string(500, 16, "CS:RIP", &COLOR_WHITE);
        print_hex(f.cs, 4, 500 + 8 * 7, 16);
        print_hex(f.rip, 16, 500 + 8 * 12, 16);

        w.write_string(500, 16 * 2, "RFLAGS", &COLOR_WHITE);
        print_hex(f.rflags, 16, 500 + 8 * 7, 16 * 2);

        w.write_string(500, 16 * 3, "SS:RSP", &COLOR_WHITE);
        print_hex(f.ss, 16, 500 + 8 * 7, 16 * 3);
        print_hex(f.rsp, 16, 500 + 8 * 12, 16 * 3);
    }

    pub fn print_hex(value: u64, width: i32, pos_x: i32, pos_y: i32) {
        for i in 0..width {
            let mut x = value >> (4 * (width - i - 1)) & 0xf;
            if x >= 10 {
                x += u64::from('a') - 10;
            } else {
                x += u64::from('0');
            }

            pixel_writer().write_ascii(pos_x + (8 * i), pos_y, char::from(x as u8), &COLOR_WHITE);
        }
    }
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
    pub fn set_idt_entry(&mut self, offset: usize) {
        self._set_idt_entry(
            InterruptDescriptorAttribute::new(SystemDescriptorType::InterruptGate, 0, true, 0),
            offset as u64,
            KERNEL_CS,
        );
    }

    pub fn _set_idt_entry(
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
    LAPICTimer = 0x41,
}
