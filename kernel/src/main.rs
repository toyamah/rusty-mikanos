#![allow(dead_code)]
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use crate::usb::global::xhci_controller;
use alloc::format;
use alloc::string::ToString;
use core::arch::asm;
use core::panic::PanicInfo;
use lib::acpi::Rsdp;
use lib::asm::global::get_cr3;
use lib::graphics::global::{frame_buffer_config, screen_size};
use lib::graphics::{
    fill_rectangle, PixelColor, PixelWriter, Rectangle, Vector2D, COLOR_BLACK, COLOR_WHITE,
};
use lib::interrupt::global::{initialize_interrupt, notify_end_of_interrupt};
use lib::interrupt::InterruptFrame;
use lib::layer::global::{layer_manager, screen_frame_buffer};
use lib::message::{Message, MessageType};
use lib::mouse::global::mouse;
use lib::task::global::task_manager;
use lib::timer::global::{lapic_timer_on_interrupt, timer_manager};
use lib::timer::{Timer, TIMER_FREQ};
use lib::window::Window;
use lib::{
    acpi, console, graphics, keyboard, layer, memory_manager, mouse, paging, pci, segment, task,
    timer,
};
use memory_allocator::MemoryAllocator;
use shared::{FrameBufferConfig, MemoryMap};

mod logger;
mod memory_allocator;
mod usb;

static mut MAIN_WINDOW: Option<Window> = None;
fn main_window() -> &'static mut Window {
    unsafe { MAIN_WINDOW.as_mut().unwrap() }
}
fn main_window_ref() -> &'static Window {
    unsafe { MAIN_WINDOW.as_ref().unwrap() }
}

static mut MAIN_WINDOW_LAYER_ID: Option<u32> = None;
fn main_window_layer_id() -> u32 {
    unsafe { MAIN_WINDOW_LAYER_ID.unwrap() }
}

static mut TEXT_WINDOW: Option<Window> = None;
fn text_window() -> &'static mut Window {
    unsafe { TEXT_WINDOW.as_mut().unwrap() }
}
fn text_window_ref() -> &'static Window {
    unsafe { TEXT_WINDOW.as_ref().unwrap() }
}

static mut TEXT_WINDOW_LAYER_ID: Option<u32> = None;
fn text_window_layer_id() -> u32 {
    unsafe { TEXT_WINDOW_LAYER_ID.unwrap() }
}

static mut TEXT_WINDOW_INDEX: i32 = 0;
fn text_window_index() -> i32 {
    unsafe { TEXT_WINDOW_INDEX }
}

static mut TASK_B_WINDOW: Option<Window> = None;
fn task_b_window() -> &'static mut Window {
    unsafe { TASK_B_WINDOW.as_mut().unwrap() }
}
fn task_b_window_ref() -> &'static Window {
    unsafe { TASK_B_WINDOW.as_ref().unwrap() }
}

static mut TASK_B_WINDOW_LAYER_ID: Option<u32> = None;
fn task_b_window_layer_id() -> u32 {
    unsafe { TASK_B_WINDOW_LAYER_ID.unwrap() }
}

#[repr(align(16))]
struct KernelMainStack([u8; 1024 * 1024]);

#[no_mangle]
static mut KERNEL_MAIN_STACK: KernelMainStack = KernelMainStack([0; 1024 * 1024]);

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMainNewStack(
    frame_buffer_config_: &'static FrameBufferConfig,
    memory_map: &'static MemoryMap,
    acpi_table: &'static Rsdp,
) -> ! {
    let memory_map = *memory_map;
    graphics::global::initialize(*frame_buffer_config_);
    console::global::initialize();

    printk!("Welcome to MikanOS!\n");
    logger::init(log::LevelFilter::Trace).unwrap();

    segment::global::initialize();
    paging::global::initialize();
    memory_manager::global::initialize(&memory_map);
    initialize_interrupt(int_handler_xhci as usize, int_handler_lapic_timer as usize);

    pci::initialize();
    usb::register_mouse_observer(mouse_observer);

    layer::global::initialize();
    initialize_main_window();
    initialize_text_window();
    initialize_task_b_window();
    layer_manager().draw_on(
        Rectangle::new(Vector2D::new(0, 0), screen_size().to_i32_vec2d()),
        screen_frame_buffer(),
    );

    acpi::global::initialize(acpi_table);
    timer::global::initialize_lapic_timer();

    let text_box_cursor_timer = 1;
    let timer_05_sec = TIMER_FREQ / 2;
    unsafe { asm!("cli") };
    timer_manager().add_timer(Timer::new(timer_05_sec, text_box_cursor_timer));
    unsafe { asm!("sti") };
    let mut text_box_cursor_visible = false;

    task::global::initialize();
    let main_task_id = task_manager().main_task_mut().id();
    let task_b_id = task_manager()
        .new_task()
        .init_context(task_b, 45, get_cr3)
        .id();
    task_manager().wake_up(task_b_id).unwrap();

    usb::global::initialize();
    usb::register_keyboard_observer(keyboard_observer);
    mouse::global::initialize();

    loop {
        fill_rectangle(
            main_window().writer(),
            &Vector2D::new(24, 28),
            &Vector2D::new(8 * 10, 16),
            &PixelColor::new(0xc6, 0xc6, 0xc6),
        );
        let tick = unsafe { timer_manager().current_tick_with_lock() };
        main_window().write_string(24, 28, &format!("{:010}", tick), &COLOR_WHITE);
        layer_manager().draw_layer_of(main_window_layer_id(), screen_frame_buffer());

        // prevent int_handler_xhci method from taking an interrupt to avoid part of data racing of main queue.
        unsafe { asm!("cli") }; // set Interrupt Flag of CPU 0
        let message = task_manager().main_task_mut().receive_message();
        if message.is_none() {
            task_manager().sleep(main_task_id).unwrap();
            unsafe { asm!("sti") }; // next interruption event makes CPU get back from power save mode.
            continue;
        }
        let message = message.unwrap();

        unsafe { asm!("sti") }; // set CPU Interrupt Flag 1
        match message.m_type {
            MessageType::InterruptXhci => xhci_controller().process_events(),
            MessageType::TimerTimeout { timeout, value } => {
                if value == text_box_cursor_timer {
                    unsafe { asm!("cli") };
                    timer_manager()
                        .add_timer(Timer::new(timeout + timer_05_sec, text_box_cursor_timer));
                    unsafe { asm!("sti") };
                    text_box_cursor_visible = !text_box_cursor_visible;
                    draw_text_cursor(text_box_cursor_visible);
                    layer_manager().draw_layer_of(text_window_layer_id(), screen_frame_buffer());
                }
            }
            MessageType::KeyPush {
                modifier: _,
                keycode: _,
                ascii,
            } => {
                input_text_window(ascii);
                if ascii == 's' {
                    let str = task_manager()
                        .sleep(task_b_id)
                        .map(|_| "Success".to_string())
                        .unwrap_or_else(|e| e.to_string());
                    printk!("sleep taskB: {}\n", str)
                } else if ascii == 'w' {
                    let str = task_manager()
                        .wake_up(task_b_id)
                        .map(|_| "Success".to_string())
                        .unwrap_or_else(|e| e.to_string());
                    printk!("wake up taskB: {}\n", str)
                }
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

extern "C" fn keyboard_observer(modifier: u8, keycode: u8) {
    keyboard::on_input(modifier, keycode, task_manager());
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

extern "x86-interrupt" fn int_handler_lapic_timer(_: *const InterruptFrame) {
    lapic_timer_on_interrupt(task_manager());
    notify_end_of_interrupt();
}

fn initialize_main_window() {
    unsafe { MAIN_WINDOW = Some(Window::new(160, 52, frame_buffer_config().pixel_format)) }
    main_window().draw_window("hello window");
    let main_window_layer_id = layer_manager()
        .new_layer()
        .set_window(main_window_ref())
        .set_draggable(true)
        .move_(Vector2D::new(300, 100))
        .id();
    layer_manager().up_down(main_window_layer_id, 2);

    unsafe { MAIN_WINDOW_LAYER_ID = Some(main_window_layer_id) };
}

fn initialize_text_window() {
    let win_w = 160;
    let win_h = 52;

    unsafe {
        TEXT_WINDOW = Some(Window::new(
            win_w,
            win_h,
            frame_buffer_config().pixel_format,
        ))
    }

    text_window().draw_window("Text Box Test");
    text_window().draw_text_box(
        Vector2D::new(4, 24),
        Vector2D::new(win_w as i32 - 8, win_h as i32 - 24 - 4),
    );

    let id = layer_manager()
        .new_layer()
        .set_window(text_window())
        .set_draggable(true)
        .move_(Vector2D::new(350, 200))
        .id();
    unsafe { TEXT_WINDOW_LAYER_ID = Some(id) };

    layer_manager().up_down(id, i32::MAX);
}

fn input_text_window(c: char) {
    if c == '\0' {
        return;
    }

    fn pos() -> Vector2D<i32> {
        Vector2D::new(8 + 8 * text_window_index(), 24 + 6)
    }

    let max_chars = (text_window_ref().width() - 16) / 8 - 1;
    if c == '\x08' && text_window_index() > 0 {
        draw_text_cursor(false);
        unsafe { TEXT_WINDOW_INDEX -= 1 };
        fill_rectangle(
            text_window().writer(),
            &pos(),
            &Vector2D::new(8, 16),
            &PixelColor::from(0xffffff),
        );
        draw_text_cursor(true);
    } else if c >= ' ' && text_window_index() < max_chars {
        draw_text_cursor(false);
        let pos = pos();
        text_window()
            .writer()
            .write_ascii(pos.x, pos.y, c, &COLOR_WHITE);
        unsafe { TEXT_WINDOW_INDEX += 1 };
        draw_text_cursor(true);
    }

    layer_manager().draw_layer_of(text_window_layer_id(), screen_frame_buffer());
}

fn initialize_task_b_window() {
    unsafe { TASK_B_WINDOW = Some(Window::new(160, 52, frame_buffer_config().pixel_format)) };
    task_b_window().draw_window("TaskB Window");

    let layer_id = layer_manager()
        .new_layer()
        .set_window(task_b_window_ref())
        .set_draggable(true)
        .move_(Vector2D::new(100, 100))
        .id();
    unsafe { TASK_B_WINDOW_LAYER_ID = Some(layer_id) };

    layer_manager().up_down(layer_id, i32::MAX);
}

fn task_b(task_id: u64, data: usize) {
    printk!("TaskB: task_id ={}, data={}\n", task_id, data);
    for i in 0.. {
        fill_rectangle(
            task_b_window().writer(),
            &Vector2D::new(24, 28),
            &Vector2D::new(8 * 10, 16),
            &PixelColor::new(0xc6, 0xc6, 0xc6),
        );
        task_b_window()
            .writer()
            .write_string(24, 28, format!("{:010}", i).as_str(), &COLOR_WHITE);
        layer_manager().draw_layer_of(task_b_window_layer_id(), screen_frame_buffer());
    }
}

fn draw_text_cursor(visible: bool) {
    let color = if visible { &COLOR_WHITE } else { &COLOR_BLACK };
    let pos = Vector2D::new(8 + 8 * text_window_index(), 24 + 5);
    fill_rectangle(text_window().writer(), &pos, &Vector2D::new(7, 15), color);
}

fn loop_and_hlt() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => ($crate::console::global::_printk(format_args!($($arg)*)));
}
