use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();

    build_usb(&out_dir, &current_dir);

    // It allows Rust to include libusb.a in the elf.
    println!("cargo:rustc-link-search=native={}", out_dir);
}

fn build_usb(out_dir: &str, current_dir: &Path) {
    let usb_dir = Path::new(current_dir).join("usb");

    Command::new("make").current_dir(&usb_dir).status().unwrap();

    fs::copy(
        PathBuf::from(&usb_dir).join("libusb.a"),
        Path::new(out_dir).join("libusb.a"),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=usb");
    println!("cargo:rustc-link-lib=static=usb");
}
