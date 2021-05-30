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

use crate::console::Console;
use crate::graphics::{
    PixelColor, PixelWriter, Vector2D, COLOR_BLACK, COLOR_WHITE, DESKTOP_BG_COLOR, DESKTOP_FG_COLOR,
};
use crate::pci::Device;
use core::panic::PanicInfo;
use log::{debug, error, info, trace};
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

    let result = match pci::scan_all_bus() {
        Ok(_) => "Success",
        Err(error) => error.name(),
    };
    printk!("ScannAllBus: {}\n", result);

    for device in pci::devices() {
        printk!("{}\n", device);
    }

    let device = pci::devices()
        .iter()
        .find(|d| d.is_xhc() && d.is_intel_device())
        .or_else(|| pci::devices().iter().find(|d| d.is_xhc()))
        .unwrap_or_else(|| {
            info!("no xHC has been found");
            loop_and_hlt()
        });

    info!("xHC has been found: {}", device);

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
