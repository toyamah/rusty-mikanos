#![allow(dead_code)]
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::format;
use bit_field::BitField;
use core::arch::asm;
use core::panic::PanicInfo;
use lib::asm::{set_csss, set_ds_all};
use lib::error::Error;
use lib::graphics::global::{frame_buffer_config, pixel_writer, screen_size};
use lib::graphics::{
    draw_desktop, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_WHITE,
};
use lib::interrupt::{initialize_interrupt, notify_end_of_interrupt, InterruptFrame};
use lib::layer::global::{layer_manager, screen_frame_buffer};
use lib::message::{Message, MessageType};
use lib::mouse::{draw_mouse_cursor, new_mouse_cursor_window};
use lib::paging::setup_identity_page_table;
use lib::pci::Device;
use lib::segment::set_up_segment;
use lib::timer::initialize_api_timer;
use lib::window::Window;
use lib::{console, graphics, interrupt, layer, memory_manager, pci};
use log::{error, info};
use memory_allocator::MemoryAllocator;
use shared::{FrameBufferConfig, MemoryMap};
use usb::XhciController;

mod logger;
mod memory_allocator;
mod usb;

static mut MAIN_QUEUE: Option<VecDeque<Message>> = None;
pub fn main_queue() -> &'static mut VecDeque<Message> {
    unsafe { MAIN_QUEUE.as_mut().unwrap() }
}

static mut XHCI_CONTROLLER: Option<XhciController> = None;
fn xhci_controller() -> &'static mut XhciController {
    unsafe { XHCI_CONTROLLER.as_mut().unwrap() }
}

static mut MOUSE_CURSOR_WINDOW: Option<Window> = None;
fn mouse_cursor_window() -> &'static mut Window {
    unsafe { MOUSE_CURSOR_WINDOW.as_mut().unwrap() }
}
fn mouse_cursor_window_ref() -> &'static Window {
    unsafe { MOUSE_CURSOR_WINDOW.as_ref().unwrap() }
}

static mut MAIN_WINDOW: Option<Window> = None;
fn main_window() -> &'static mut Window {
    unsafe { MAIN_WINDOW.as_mut().unwrap() }
}
fn main_window_ref() -> &'static Window {
    unsafe { MAIN_WINDOW.as_ref().unwrap() }
}

static mut MOUSE_LAYER_ID: u32 = u32::MAX;
fn mouse_layer_id() -> u32 {
    unsafe { MOUSE_LAYER_ID }
}

static mut MOUSE_POSITION: Vector2D<usize> = Vector2D::new(200, 200);
fn mouse_position() -> Vector2D<usize> {
    unsafe { MOUSE_POSITION }
}

#[repr(align(16))]
struct KernelMainStack([u8; 1024 * 1024]);

#[no_mangle]
static mut KERNEL_MAIN_STACK: KernelMainStack = KernelMainStack([0; 1024 * 1024]);

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMainNewStack(
    frame_buffer_config_: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
) -> ! {
    let memory_map = *memory_map;
    graphics::global::initialize(*frame_buffer_config_);
    initialize_global_vars();
    console::initialize();
    draw_desktop(pixel_writer());
    printk!("Welcome to MikanOS!\n");
    initialize_api_timer();

    let kernel_cs: u16 = 1 << 3;
    let kernel_ss: u16 = 2 << 3;
    set_up_segment();
    set_ds_all(0);
    set_csss(kernel_cs, kernel_ss);
    setup_identity_page_table();

    memory_manager::global::initialize(&memory_map);
    pci::scan_all_bus().unwrap();

    // for device in pci::devices() {
    //     printk!("{}\n", device);
    // }

    let xhc_device = pci::find_xhc_device().unwrap_or_else(|| {
        info!("no xHC has been found");
        loop_and_hlt()
    });

    initialize_interrupt(int_handler_xhci as usize, kernel_cs);

    enable_to_interrupt_for_xhc(xhc_device).unwrap();

    let xhc_bar = pci::read_bar(xhc_device, 0).unwrap_or_else(|e| {
        info!("cannot read base address#0: {}", e);
        loop_and_hlt()
    });
    let xhc_mmio_base = xhc_bar & !(0x0f_u64);
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

    layer::global::initialize();

    unsafe {
        MOUSE_CURSOR_WINDOW = Some(new_mouse_cursor_window(frame_buffer_config().pixel_format))
    }
    draw_mouse_cursor(mouse_cursor_window().writer(), &Vector2D::new(0, 0));

    unsafe { MAIN_WINDOW = Some(Window::new(160, 52, frame_buffer_config().pixel_format)) }
    main_window().draw_window("hello window");

    let main_window_layer_id = layer_manager()
        .new_layer()
        .set_window(main_window_ref())
        .set_draggable(true)
        .move_(Vector2D::new(300, 100))
        .id();
    {
        let id = layer_manager()
            .new_layer()
            .set_window(mouse_cursor_window_ref())
            .move_(mouse_position().to_i32_vec2d())
            .id();
        unsafe { MOUSE_LAYER_ID = id }
    }

    layer_manager().up_down(main_window_layer_id, 2);
    layer_manager().up_down(mouse_layer_id(), 3);
    layer_manager().draw_on(
        Rectangle::new(Vector2D::new(0, 0), screen_size().to_i32_vec2d()),
        screen_frame_buffer(),
    );

    let mut count = 0;
    loop {
        count += 1;
        fill_rectangle(
            main_window().writer(),
            &Vector2D::new(24, 28),
            &Vector2D::new(8 * 10, 16),
            &PixelColor::new(0xc6, 0xc6, 0xc6),
        );
        main_window().write_string(24, 28, &format!("{:010}", count), &COLOR_WHITE);
        layer_manager().draw_layer_of(main_window_layer_id, screen_frame_buffer());

        // prevent int_handler_xhci method from taking an interrupt to avoid part of data racing of main queue.
        unsafe { asm!("cli") }; // set Interrupt Flag of CPU 0
        if main_queue().is_empty() {
            // next interruption event makes CPU get back from power save mode.
            unsafe {
                asm!("sti");
                // asm!("sti\n\thlt"); // execute sti and then hlt
            };
            continue;
        }

        let result = main_queue().pop_back();
        unsafe {
            asm!("sti"); // set CPU Interrupt Flag 1
        };
        match result {
            Some(Message {
                m_type: MessageType::KInterruptXhci,
            }) => {
                while xhci_controller().primary_event_ring_has_front() {
                    if let Err(code) = xhci_controller().process_event() {
                        error!("Error while ProcessEvent: {}", code)
                    }
                }
            }
            None => error!("failed to pop a message from MainQueue."),
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

static mut MOUSE_DRAG_LAYER_ID: u32 = 0;
fn mouse_drag_layer_id() -> u32 {
    unsafe { MOUSE_DRAG_LAYER_ID }
}

static mut PREVIOUS_BUTTONS: u8 = 0;
fn previous_buttons() -> u8 {
    unsafe { PREVIOUS_BUTTONS }
}

extern "C" fn mouse_observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    let new_pos = mouse_position().to_i32_vec2d()
        + Vector2D::new(displacement_x as i32, displacement_y as i32);
    let new_pos = new_pos
        .element_min(screen_size().to_i32_vec2d() + Vector2D::new(-1, -1))
        .element_max(Vector2D::new(0, 0));

    let old_pos = mouse_position();
    unsafe { MOUSE_POSITION = Vector2D::new(new_pos.x as usize, new_pos.y as usize) }
    let pos_diff = mouse_position() - old_pos;
    layer_manager().move_(mouse_layer_id(), new_pos, screen_frame_buffer());

    let previous_left_pressed = (previous_buttons() & 0x01) != 0;
    let left_pressed = (buttons & 0x01) != 0;
    if !previous_left_pressed && left_pressed {
        let draggable_layer = layer_manager()
            .find_layer_by_position(new_pos, mouse_layer_id())
            .filter(|l| l.is_draggable());
        if let Some(l) = draggable_layer {
            unsafe { MOUSE_DRAG_LAYER_ID = l.id() }
        }
    } else if previous_left_pressed && left_pressed {
        if mouse_drag_layer_id() > 0 {
            layer_manager().move_relative(
                mouse_drag_layer_id(),
                pos_diff.to_i32_vec2d(),
                screen_frame_buffer(),
            );
        }
    } else if previous_left_pressed && !left_pressed {
        unsafe { MOUSE_DRAG_LAYER_ID = 0 };
    }

    unsafe { PREVIOUS_BUTTONS = buttons };
}

extern "x86-interrupt" fn int_handler_xhci(_: *const InterruptFrame) {
    main_queue().push_front(Message::new(MessageType::KInterruptXhci));
    notify_end_of_interrupt();
}

fn initialize_global_vars() {
    unsafe {
        MAIN_QUEUE = Some(VecDeque::new());
    }

    usb::register_mouse_observer(mouse_observer);

    logger::init(log::LevelFilter::Trace).unwrap();
}

fn loop_and_hlt() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
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

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ($crate::console::_printk(format_args!($($arg)*)));
}
