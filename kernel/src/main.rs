#![allow(dead_code)]
#![feature(asm)]
#![feature(llvm_asm)]
#![no_std]
#![no_main]

mod console;
mod error;
mod font;
mod graphics;
mod logger;
mod pci;
mod usb;

use crate::console::Console;
use crate::graphics::{
    PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE, DESKTOP_BG_COLOR, DESKTOP_FG_COLOR,
};
use crate::usb::XhciController;
use core::panic::PanicInfo;
use log::{debug, error, info};
use shared::FrameBufferConfig;

static mut PIXEL_WRITER: Option<PixelWriter> = None;

fn pixel_writer() -> &'static mut PixelWriter<'static> {
    unsafe { PIXEL_WRITER.as_mut().unwrap() }
}

static mut CONSOLE: Option<Console> = None;

fn console() -> &'static mut Console<'static> {
    unsafe { CONSOLE.as_mut().unwrap() }
}

#[no_mangle] // disable name mangling
pub extern "C" fn KernelMain(frame_buffer_config: &'static FrameBufferConfig) -> ! {
    initialize_global_vars(frame_buffer_config);

    let frame_width = frame_buffer_config.horizontal_resolution;
    let frame_height = frame_buffer_config.vertical_resolution;
    let writer = pixel_writer();
    writer.fill_rectangle(
        &Vector2D::new(0, 0),
        &Vector2D::new(frame_width, frame_height),
        &DESKTOP_BG_COLOR,
    );
    writer.fill_rectangle(
        &Vector2D::new(0, frame_height - 50),
        &Vector2D::new(frame_width, 50),
        &PixelColor::new(1, 8, 17),
    );
    writer.fill_rectangle(
        &Vector2D::new(0, frame_height - 50),
        &Vector2D::new(frame_width / 5, 50),
        &PixelColor::new(80, 80, 80),
    );
    writer.draw_rectange(
        &Vector2D::new(10, frame_height - 40),
        &Vector2D::new(30, 30),
        &PixelColor::new(160, 160, 160),
    );

    printk!("Welcome to MikanOS!\n");
    write_cursor();

    pci::scan_all_bus().unwrap();

    for device in pci::devices() {
        printk!("{}\n", device);
    }

    let xhc_device = pci::find_xhc_device().unwrap_or_else(|| {
        info!("no xHC has been found");
        loop_and_hlt()
    });
    info!("xHC has been found: {}", xhc_device);

    let xhc_bar = pci::read_bar(xhc_device, 0).unwrap_or_else(|e| {
        info!("cannot read base address#0: {}", e);
        loop_and_hlt()
    });
    let xhc_mmio_base = xhc_bar & !(0x0f as u64);
    debug!("xHC mmio_base = {:08x}", xhc_mmio_base);

    let controller = XhciController::new(xhc_mmio_base);
    if xhc_device.is_intel_device() {
        xhc_device.switch_ehci_to_xhci();
    }
    controller.initialize().unwrap();
    controller.run().unwrap();

    loop_and_hlt()
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    error!("{}", _info);
    loop_and_hlt()
}

fn initialize_global_vars(frame_buffer_config: &'static FrameBufferConfig) {
    unsafe {
        PIXEL_WRITER = Some(PixelWriter::new(frame_buffer_config));
    }

    unsafe {
        CONSOLE = Some(Console::new(
            pixel_writer(),
            DESKTOP_FG_COLOR,
            DESKTOP_BG_COLOR,
        ));
    }

    logger::init(log::LevelFilter::Trace).unwrap();
}

fn write_cursor() {
    let writer = pixel_writer();
    for (dy, str) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, char) in str.chars().enumerate() {
            if char == '@' {
                writer.write((200 + dx) as u32, (100 + dy) as u32, &COLOR_WHITE);
            } else if char == '.' {
                writer.write((200 + dx) as u32, (100 + dy) as u32, &COLOR_BLACK);
            };
        }
    }
}

fn loop_and_hlt() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}

const MOUSE_CURSOR_SHAPE: [&str; 24] = [
    "@              ",
    "@@             ",
    "@.@            ",
    "@..@           ",
    "@...@          ",
    "@....@         ",
    "@.....@        ",
    "@......@       ",
    "@.......@      ",
    "@........@     ",
    "@.........@    ",
    "@..........@   ",
    "@...........@  ",
    "@............@ ",
    "@......@@@@@@@@",
    "@......@       ",
    "@....@@.@      ",
    "@...@ @.@      ",
    "@..@   @.@     ",
    "@.@    @.@     ",
    "@@      @.@    ",
    "@       @.@    ",
    "         @.@   ",
    "         @@@   ",
];
