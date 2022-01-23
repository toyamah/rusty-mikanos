#![allow(dead_code)]
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

mod asm;
mod console;
mod error;
mod font;
mod graphics;
mod interrupt;
mod layer;
mod logger;
mod memory_allocator;
mod memory_manager;
mod memory_map;
mod mouse;
mod paging;
mod pci;
mod queue;
mod segment;
mod usb;
mod window;
mod x86_descriptor;

extern crate alloc;

use crate::asm::{set_csss, set_ds_all};
use crate::console::Console;
use crate::error::Error;
use crate::graphics::{
    draw_rectangle, fill_rectangle, FrameBufferWriter, PixelColor, PixelWriter, Vector2D,
    DESKTOP_BG_COLOR, DESKTOP_FG_COLOR,
};
use crate::interrupt::setup_idt;
use crate::layer::LayerManager;
use crate::memory_allocator::MemoryAllocator;
use crate::memory_manager::{BitmapMemoryManager, FrameID, BYTES_PER_FRAME};
use crate::memory_map::UEFI_PAGE_SIZE;
use crate::mouse::{draw_mouse_cursor, new_mouse_cursor_window};
use crate::paging::setup_identity_page_table;
use crate::pci::Device;
use crate::queue::ArrayQueue;
use crate::segment::set_up_segment;
use crate::usb::XhciController;
use crate::window::Window;
use bit_field::BitField;
use core::arch::asm;
use core::borrow::BorrowMut;
use core::panic::PanicInfo;
use log::{error, info};
use shared::{FrameBufferConfig, MemoryDescriptor, MemoryMap};

static mut PIXEL_WRITER: Option<FrameBufferWriter> = None;

fn pixel_writer() -> &'static mut FrameBufferWriter<'static> {
    unsafe { PIXEL_WRITER.as_mut().unwrap() }
}

static mut CONSOLE: Option<Console> = None;

fn console() -> &'static mut Console<'static> {
    unsafe { CONSOLE.as_mut().unwrap() }
}

static mut XHCI_CONTROLLER: Option<XhciController> = None;

fn xhci_controller() -> &'static mut XhciController {
    unsafe { XHCI_CONTROLLER.as_mut().unwrap() }
}

static mut FRAME_BUFFER_CONFIG: Option<FrameBufferConfig> = None;

fn frame_buffer_config() -> &'static mut FrameBufferConfig {
    unsafe { FRAME_BUFFER_CONFIG.as_mut().unwrap() }
}

static mut MEMORY_MANAGER: BitmapMemoryManager = BitmapMemoryManager::new();

fn memory_manager() -> &'static mut BitmapMemoryManager {
    unsafe { MEMORY_MANAGER.borrow_mut() }
}

static mut LAYER_MANAGER: Option<LayerManager> = None;
fn layer_manager() -> &'static mut LayerManager<'static> {
    unsafe { LAYER_MANAGER.as_mut().unwrap() }
}

static mut BG_WINDOW: Option<Window> = None;
fn bg_window() -> &'static mut Window {
    unsafe { BG_WINDOW.as_mut().unwrap() }
}
fn bg_window_ref() -> &'static Window {
    unsafe { BG_WINDOW.as_ref().unwrap() }
}

static mut MOUSE_CURSOR_WINDOW: Option<Window> = None;
fn mouse_cursor_window() -> &'static mut Window {
    unsafe { MOUSE_CURSOR_WINDOW.as_mut().unwrap() }
}
fn mouse_cursor_window_ref() -> &'static Window {
    unsafe { MOUSE_CURSOR_WINDOW.as_ref().unwrap() }
}

static mut MOUSE_LAYER_ID: u32 = u32::MAX;
fn mouse_layer_id() -> u32 {
    unsafe { MOUSE_LAYER_ID }
}

#[derive(Copy, Clone, Debug)]
struct Message {
    m_type: MessageType,
}

#[derive(Copy, Clone, Debug)]
enum MessageType {
    KInterruptXhci,
}

static mut MESSAGES: [Message; 32] = [Message {
    m_type: MessageType::KInterruptXhci,
}; 32];
static mut MAIN_QUEUE: Option<ArrayQueue<Message, 32>> = None;

fn main_queue() -> &'static mut ArrayQueue<'static, Message, 32> {
    unsafe { MAIN_QUEUE.as_mut().unwrap() }
}

#[repr(align(16))]
pub struct KernelMainStack([u8; 1024 * 1024]);

#[no_mangle]
static mut KERNEL_MAIN_STACK: KernelMainStack = KernelMainStack([0; 1024 * 1024]);

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMainNewStack(
    frame_buffer_config_: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
) -> ! {
    unsafe { FRAME_BUFFER_CONFIG = Some(*frame_buffer_config_) }
    let memory_map = *memory_map;
    initialize_global_vars(frame_buffer_config());
    draw_background(pixel_writer());
    printk!("Welcome to MikanOS!\n");

    let kernel_cs: u16 = 1 << 3;
    let kernel_ss: u16 = 2 << 3;
    set_up_segment();
    set_ds_all(0);
    set_csss(kernel_cs, kernel_ss);
    setup_identity_page_table();

    let buffer = memory_map.buffer as usize;
    let mut available_end = 0;
    let mut iter = buffer;
    while iter < buffer + memory_map.map_size as usize {
        let desc = iter as *const MemoryDescriptor;
        let physical_start = unsafe { (*desc).physical_start };
        let number_of_pages = unsafe { (*desc).number_of_pages };
        if available_end < physical_start {
            memory_manager().mark_allocated(
                FrameID::new(available_end / BYTES_PER_FRAME),
                (physical_start - available_end) / BYTES_PER_FRAME,
            );
        }

        let type_ = unsafe { &(*desc).type_ };
        let byte_count = (number_of_pages * UEFI_PAGE_SIZE as u64) as usize;
        let physical_end = physical_start + byte_count;
        if type_.is_available() {
            available_end = physical_end;
        } else {
            memory_manager().mark_allocated(
                FrameID::new(physical_start / BYTES_PER_FRAME),
                byte_count / BYTES_PER_FRAME as usize,
            )
        }
        iter += memory_map.descriptor_size as usize;
    }
    memory_manager().set_memory_range(
        FrameID::new(1),
        FrameID::new(available_end / BYTES_PER_FRAME),
    );

    pci::scan_all_bus().unwrap();

    // for device in pci::devices() {
    //     printk!("{}\n", device);
    // }

    let xhc_device = pci::find_xhc_device().unwrap_or_else(|| {
        info!("no xHC has been found");
        loop_and_hlt()
    });

    setup_idt(int_handler_xhci as u64, kernel_cs);

    enable_to_interrupt_for_xhc(xhc_device).unwrap();

    let xhc_bar = pci::read_bar(xhc_device, 0).unwrap_or_else(|e| {
        info!("cannot read base address#0: {}", e);
        loop_and_hlt()
    });
    let xhc_mmio_base = xhc_bar & !(0x0f as u64);
    // debug!("xHC mmio_base = {:08x}", xhc_mmio_base);

    let controller = XhciController::new(xhc_mmio_base);
    if xhc_device.is_intel_device() {
        xhc_device.switch_ehci_to_xhci();
    }
    controller.initialize().unwrap();
    controller.run().unwrap();

    unsafe {
        XHCI_CONTROLLER = Some(controller);
        asm!("sti");
    };

    xhci_controller().configure_port();

    unsafe {
        BG_WINDOW = Some(Window::new(
            frame_buffer_config_.horizontal_resolution as usize,
            frame_buffer_config_.vertical_resolution as usize,
        ))
    }
    draw_background(bg_window().writer());
    console().reset_writer(bg_window().writer());

    unsafe { MOUSE_CURSOR_WINDOW = Some(new_mouse_cursor_window()) }
    draw_mouse_cursor(mouse_cursor_window().writer(), &Vector2D::new(0, 0));

    unsafe { LAYER_MANAGER = Some(LayerManager::new(pixel_writer())) }
    let bg_layer_id = layer_manager()
        .new_layer()
        .set_window(bg_window_ref())
        .move_(Vector2D::new(0, 0))
        .id();
    {
        let id = layer_manager()
            .new_layer()
            .set_window(mouse_cursor_window_ref())
            .move_(Vector2D::new(200, 200))
            .id();
        unsafe { MOUSE_LAYER_ID = id }
    }

    layer_manager().up_down(bg_layer_id, 0);
    layer_manager().up_down(mouse_layer_id(), 1);
    layer_manager().draw();

    loop {
        // prevent int_handler_xhci method from taking an interrupt to avoid part of data racing of main queue.
        unsafe { asm!("cli") }; // set Interrupt Flag of CPU 0
        if main_queue().count() == 0 {
            // next interruption event makes CPU get back from power save mode.
            unsafe {
                asm!("sti\n\thlt"); // execute sti and then hlt
            };
            continue;
        }

        let result = main_queue().pop();
        unsafe {
            asm!("sti"); // set CPU Interrupt Flag 1
        };
        match result {
            Ok(Message {
                m_type: MessageType::KInterruptXhci,
            }) => {
                while xhci_controller().primary_event_ring_has_front() {
                    match xhci_controller().process_event() {
                        Err(code) => error!("Error while ProcessEvent: {}", code),
                        Ok(_) => {}
                    }
                }
            }
            Err(error) => {
                error!("failed to pop a message from MainQueue. {}", error)
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    printk!("{}", _info); // Use printk to show the entire message
    loop_and_hlt()
}

#[global_allocator]
static ALLOCATOR: MemoryAllocator = MemoryAllocator;

#[alloc_error_handler]
fn alloc_error_handle(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

extern "C" fn mouse_observer(displacement_x: i8, displacement_y: i8) {
    layer_manager().move_relative(
        mouse_layer_id(),
        Vector2D::new(displacement_x as i32, displacement_y as i32),
    );
    layer_manager().draw();
}

extern "x86-interrupt" fn int_handler_xhci(_: *const interrupt::InterruptFrame) {
    main_queue()
        .push(Message {
            m_type: MessageType::KInterruptXhci,
        })
        .unwrap_or_else(|e| error!("failed to push a Message to main_queue {}", e));

    interrupt::notify_end_of_interrupt();
}

fn initialize_global_vars(frame_buffer_config: &'static FrameBufferConfig) {
    unsafe {
        PIXEL_WRITER = Some(FrameBufferWriter::new(frame_buffer_config));

        CONSOLE = Some(Console::new(
            pixel_writer(),
            DESKTOP_FG_COLOR,
            DESKTOP_BG_COLOR,
        ));

        MAIN_QUEUE = Some(ArrayQueue::new(&mut MESSAGES));
    }

    usb::register_mouse_observer(mouse_observer);

    logger::init(log::LevelFilter::Trace).unwrap();
}

fn loop_and_hlt() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

fn draw_background<W: PixelWriter>(writer: &W) {
    let width = writer.width();
    let height = writer.height();
    fill_rectangle(
        writer,
        &Vector2D::new(0, 0),
        &Vector2D::new(width, height),
        &DESKTOP_BG_COLOR,
    );
    fill_rectangle(
        writer,
        &Vector2D::new(0, height - 50),
        &Vector2D::new(width, 50),
        &PixelColor::new(1, 8, 17),
    );
    fill_rectangle(
        writer,
        &Vector2D::new(0, height - 50),
        &Vector2D::new(width / 5, 50),
        &PixelColor::new(80, 80, 80),
    );
    draw_rectangle(
        writer,
        &Vector2D::new(10, height - 40),
        &Vector2D::new(30, 30),
        &PixelColor::new(160, 160, 160),
    );
}

fn enable_to_interrupt_for_xhc(xhc_device: &Device) -> Result<(), Error> {
    // bsp is bootstrap processor which is the only core running when the power is turned on.
    let bsp_local_apic_id_addr = 0xfee00020 as *const u32;
    let bsp_local_apic_id = unsafe { (*bsp_local_apic_id_addr).get_bits(24..=31) as u8 };

    pci::configure_msi_fixed_destination(
        xhc_device,
        bsp_local_apic_id,
        pci::MsiTriggerMode::Level,
        pci::MsiDeliveryMode::Fixed,
        interrupt::InterruptVectorNumber::XHCI as u8,
        0,
    )
}
