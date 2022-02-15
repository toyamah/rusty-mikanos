#![allow(dead_code)]
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use crate::usb::global::xhci_controller;
use alloc::collections::VecDeque;
use alloc::format;
use core::arch::asm;
use core::panic::PanicInfo;
use lib::graphics::global::{frame_buffer_config, pixel_writer, screen_size};
use lib::graphics::{
    draw_desktop, fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_WHITE,
};
use lib::interrupt::{initialize_interrupt, notify_end_of_interrupt, InterruptFrame};
use lib::layer::global::{layer_manager, screen_frame_buffer};
use lib::message::{Message, MessageType};
use lib::mouse::global::mouse;
use lib::paging::setup_identity_page_table;
use lib::timer::initialize_api_timer;
use lib::window::Window;
use lib::{console, graphics, layer, memory_manager, mouse, pci, segment};
use log::error;
use memory_allocator::MemoryAllocator;
use shared::{FrameBufferConfig, MemoryMap};

mod logger;
mod memory_allocator;
mod usb;

static mut MAIN_QUEUE: Option<VecDeque<Message>> = None;
pub fn main_queue() -> &'static mut VecDeque<Message> {
    unsafe { MAIN_QUEUE.as_mut().unwrap() }
}

static mut MAIN_WINDOW: Option<Window> = None;
fn main_window() -> &'static mut Window {
    unsafe { MAIN_WINDOW.as_mut().unwrap() }
}
fn main_window_ref() -> &'static Window {
    unsafe { MAIN_WINDOW.as_ref().unwrap() }
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

    segment::global::initialize();
    setup_identity_page_table();

    memory_manager::global::initialize(&memory_map);
    initialize_interrupt(int_handler_xhci as usize);

    pci::initialize();
    usb::global::initialize();
    usb::register_mouse_observer(mouse_observer);

    layer::global::initialize();
    unsafe { MAIN_WINDOW = Some(Window::new(160, 52, frame_buffer_config().pixel_format)) }
    main_window().draw_window("hello window");
    let main_window_layer_id = layer_manager()
        .new_layer()
        .set_window(main_window_ref())
        .set_draggable(true)
        .move_(Vector2D::new(300, 100))
        .id();
    mouse::global::initialize();

    layer_manager().up_down(main_window_layer_id, 2);
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
            }) => xhci_controller().process_events(),
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

extern "C" fn mouse_observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    mouse().on_interrupt(
        buttons,
        displacement_x,
        displacement_y,
        screen_size().to_i32_vec2d(),
        layer_manager(),
        screen_frame_buffer(),
    );
}

extern "x86-interrupt" fn int_handler_xhci(_: *const InterruptFrame) {
    main_queue().push_front(Message::new(MessageType::KInterruptXhci));
    notify_end_of_interrupt();
}

fn initialize_global_vars() {
    unsafe {
        MAIN_QUEUE = Some(VecDeque::new());
    }

    logger::init(log::LevelFilter::Trace).unwrap();
}

fn loop_and_hlt() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ($crate::console::_printk(format_args!($($arg)*)));
}
