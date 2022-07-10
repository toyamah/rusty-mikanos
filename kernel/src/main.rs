#![allow(dead_code)]
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use crate::usb::global::xhci_controller;
use alloc::format;
use core::arch::asm;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicI32, Ordering};
use lib::acpi::Rsdp;
use lib::asm::global::get_cr3;
use lib::font::{write_ascii, write_string};
use lib::graphics::global::{frame_buffer_config, screen_size};
use lib::graphics::{fill_rectangle, PixelColor, Rectangle, Vector2D, COLOR_BLACK, COLOR_WHITE};
use lib::interrupt::global::initialize_interrupt;
use lib::keyboard::KEY_F2;
use lib::layer::global::{
    active_layer, get_layer_window_mut, get_layer_window_ref, layer_manager, layer_task_map,
    screen_frame_buffer,
};
use lib::layer::LayerID;
use lib::message::{Message, MessageType};
use lib::mouse::global::MOUSE;
use lib::task::global::task_manager;
use lib::task::TaskID;
use lib::terminal::lib::task_terminal;
use lib::timer::global::{current_tick_with_lock, timer_manager};
use lib::timer::{Timer, TIMER_FREQ};
use lib::window::Window;
use lib::{
    acpi, console, fat, font, graphics, keyboard, layer, memory_manager, mouse, paging, pci,
    segment, syscall, task, timer,
};
use memory_allocator::MemoryAllocator;
use shared::{FrameBufferConfig, MemoryMap};
use spin::Once;

mod logger;
mod memory_allocator;
mod usb;

fn main_window() -> &'static mut Window {
    get_layer_window_mut(main_window_layer_id()).expect("could not find main layer")
}
fn main_window_ref() -> &'static Window {
    get_layer_window_ref(main_window_layer_id()).expect("could not find main layer")
}

static mut MAIN_WINDOW_LAYER_ID: Option<LayerID> = None;
fn main_window_layer_id() -> LayerID {
    unsafe { MAIN_WINDOW_LAYER_ID.unwrap() }
}

fn text_window() -> &'static mut Window {
    get_layer_window_mut(*TEXT_WINDOW_LAYER_ID.wait()).expect("could not find text layer")
}
fn text_window_ref() -> &'static Window {
    get_layer_window_ref(*TEXT_WINDOW_LAYER_ID.wait()).expect("could not find text layer")
}

static TEXT_WINDOW_LAYER_ID: Once<LayerID> = Once::new();

static TEXT_WINDOW_INDEX: AtomicI32 = AtomicI32::new(0);
fn text_window_index() -> i32 {
    TEXT_WINDOW_INDEX.load(Ordering::SeqCst)
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
    volume_image: *mut u8,
) -> ! {
    let memory_map = *memory_map;
    graphics::global::initialize(*frame_buffer_config_);
    console::global::initialize();

    printk!("Welcome to Rusty MikanOS!\n");
    logger::init(log::LevelFilter::Trace).unwrap();

    segment::global::initialize();
    paging::global::initialize();
    memory_manager::global::initialize(&memory_map);
    segment::global::initialize_tss();
    initialize_interrupt();

    fat::global::initialize(volume_image);
    font::initialize();
    pci::initialize();
    usb::register_mouse_observer(mouse_observer);

    layer::global::initialize();
    initialize_main_window();
    initialize_text_window();
    layer_manager().draw_on(
        Rectangle::new(Vector2D::new(0, 0), screen_size().to_i32_vec2d()),
        screen_frame_buffer(),
    );

    acpi::global::initialize(acpi_table);
    timer::global::initialize_lapic_timer();

    let text_box_cursor_timer = 1;
    let timer_05_sec = TIMER_FREQ / 2;
    let expected_main_task_id = TaskID::new(0); // prepare because the main task id is created after it
    unsafe { asm!("cli") };
    timer_manager().as_mut().unwrap().add_timer(Timer::new(
        timer_05_sec,
        text_box_cursor_timer,
        expected_main_task_id,
    ));
    unsafe { asm!("sti") };
    let mut text_box_cursor_visible = false;

    syscall::initialize_syscall();

    task::global::initialize();
    let main_task_id = task_manager().main_task_mut().id();
    assert_eq!(main_task_id, expected_main_task_id);

    usb::global::initialize();
    usb::register_keyboard_observer(keyboard_observer);
    mouse::global::initialize();

    task_manager()
        .wake_up(
            task_manager()
                .new_task()
                .init_context(task_terminal, 0, get_cr3)
                .id(),
        )
        .unwrap();

    loop {
        fill_rectangle(
            main_window().writer(),
            &Vector2D::new(20, 4),
            &Vector2D::new(8 * 10, 16),
            &PixelColor::new(0xc6, 0xc6, 0xc6),
        );
        let tick = current_tick_with_lock();
        write_string(main_window(), 20, 4, &format!("{:010}", tick), &COLOR_BLACK);
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
                    timer_manager().as_mut().unwrap().add_timer(Timer::new(
                        timeout + timer_05_sec,
                        text_box_cursor_timer,
                        main_task_id,
                    ));
                    unsafe { asm!("sti") };
                    text_box_cursor_visible = !text_box_cursor_visible;
                    draw_text_cursor(text_box_cursor_visible);
                    layer_manager()
                        .draw_layer_of(*TEXT_WINDOW_LAYER_ID.wait(), screen_frame_buffer());
                }
            }
            MessageType::KeyPush(arg) => {
                let act = active_layer().get_active_layer_id();
                if act.is_none() {
                    continue;
                }
                let act = act.unwrap();

                if act == *TEXT_WINDOW_LAYER_ID.wait() && arg.press {
                    input_text_window(arg.ascii);
                } else if arg.press && arg.keycode == KEY_F2 {
                    let id = task_manager()
                        .new_task()
                        .init_context(task_terminal, 0, get_cr3)
                        .id();
                    task_manager().wake_up(id).unwrap();
                } else {
                    unsafe { asm!("cli") };
                    let task_id = layer_task_map().get(&act);
                    unsafe { asm!("sti") };
                    if let Some(&task_id) = task_id {
                        unsafe { asm!("cli") };
                        let _ = task_manager().send_message(task_id, message);
                        unsafe { asm!("sti") };
                    } else {
                        printk!(
                            "key push not handles: keycode {}, ascii {}\n",
                            arg.keycode,
                            arg.ascii
                        );
                    }
                }
            }
            MessageType::Layer(l_msg) => {
                layer_manager().process_message(&l_msg, screen_frame_buffer());
                unsafe { asm!("cli") };
                let _ = task_manager()
                    .send_message(l_msg.src_task_id, Message::new(MessageType::LayerFinish));
                unsafe { asm!("sti") };
            }
            _ => {}
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    printk!("{}\n", _info); // Use printk to show the entire message
    printk!("{:?}", _info.location().unwrap()); // Use printk to show the entire message
    loop_and_hlt()
}

#[global_allocator]
static ALLOCATOR: MemoryAllocator = MemoryAllocator;

#[alloc_error_handler]
fn alloc_error_handle(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

extern "C" fn mouse_observer(buttons: u8, displacement_x: i8, displacement_y: i8) {
    MOUSE.lock().as_mut().unwrap().on_interrupt(
        buttons,
        displacement_x,
        displacement_y,
        screen_size().to_i32_vec2d(),
        layer_manager(),
        screen_frame_buffer(),
        active_layer(),
        layer_task_map(),
        task_manager(),
    );
}

extern "C" fn keyboard_observer(modifier: u8, keycode: u8, press: bool) {
    keyboard::on_input(modifier, keycode, press, task_manager());
}

fn initialize_main_window() {
    let main_window =
        Window::new_with_title(160, 52, frame_buffer_config().pixel_format, "Hello Window");

    let main_window_layer_id = layer_manager()
        .new_layer(main_window)
        .set_draggable(true)
        .move_(Vector2D::new(300, 100))
        .id();
    layer_manager().up_down(main_window_layer_id, 2);

    unsafe { MAIN_WINDOW_LAYER_ID = Some(main_window_layer_id) };
}

fn initialize_text_window() {
    let win_w = 160;
    let win_h = 52;

    let mut text_window = Window::new_with_title(
        win_w,
        win_h,
        frame_buffer_config().pixel_format,
        "Text Box Test",
    );
    text_window.draw_text_box(Vector2D::new(0, 0), text_window.inner_size());

    let id = TEXT_WINDOW_LAYER_ID.call_once(|| {
        layer_manager()
            .new_layer(text_window)
            .set_draggable(true)
            .move_(Vector2D::new(500, 100))
            .id()
    });

    layer_manager().up_down(*id, i32::MAX);
}

fn input_text_window(c: char) {
    if c == '\0' {
        return;
    }

    fn pos() -> Vector2D<i32> {
        Vector2D::new(4 + 8 * text_window_index(), 6)
    }

    let max_chars = (text_window_ref().inner_size().x - 8) / 8 - 1;
    if c == '\x08' && text_window_index() > 0 {
        draw_text_cursor(false);
        TEXT_WINDOW_INDEX.fetch_sub(1, Ordering::SeqCst);
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
        write_ascii(text_window().writer(), pos.x, pos.y, c, &COLOR_BLACK);
        TEXT_WINDOW_INDEX.fetch_add(1, Ordering::SeqCst);
        draw_text_cursor(true);
    }

    layer_manager().draw_layer_of(*TEXT_WINDOW_LAYER_ID.wait(), screen_frame_buffer());
}

fn draw_text_cursor(visible: bool) {
    let color = if visible { &COLOR_BLACK } else { &COLOR_WHITE };
    let pos = Vector2D::new(4 + 8 * text_window_index(), 5);
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
