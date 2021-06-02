use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let current_dir = env::current_dir().unwrap();

    build_hankaku(&out_dir, &current_dir);
    build_asm(&out_dir, &current_dir);
    build_usb(&out_dir, &current_dir);

    // It allows Rust to include hankaku and other data in the elf.
    println!("cargo:rustc-link-search=native={}", out_dir);
}

fn build_hankaku(out_dir: &str, current_dir: &PathBuf) {
    let make_font = Path::new(current_dir)
        .parent()
        .unwrap()
        .join("tools")
        .join("makefont.py");
    Command::new(make_font)
        .arg("-o")
        .arg(Path::new(out_dir).join("hankaku.bin"))
        .arg("hankaku.txt")
        .current_dir(current_dir)
        .status()
        .unwrap();

    Command::new("objcopy")
        .args(&["-I", "binary", "-O", "elf64-x86-64", "-B", "i386:x86-64"])
        .arg("hankaku.bin")
        .arg("hankaku.o")
        .current_dir(out_dir)
        .status()
        .unwrap();

    // ref: https://doc.rust-lang.org/cargo/reference/build-script-examples.html#building-a-native-library
    Command::new("ar")
        .args(&["crs", "libhankaku.a", "hankaku.o"])
        .current_dir(out_dir)
        .status()
        .unwrap();

    println!("cargo:rustc-link-lib=static=hankaku");
    println!("cargo:rerun-if-changed=hankaku.txt");
}

fn build_asm(out_dir: &str, current_dir: &PathBuf) {
    // https://github.com/uchan-nos/mikanos/blob/osbook_day06b/kernel/Makefile#L37
    Command::new("nasm")
        .current_dir(current_dir)
        .args(&["-f", "elf64"])
        .arg("-o")
        .arg(Path::new(out_dir).join("asmfunc.o"))
        .arg("asmfunc.asm")
        .status()
        .unwrap();

    Command::new("ar")
        .args(&["crs", "libasmfunc.a", "asmfunc.o"])
        .current_dir(out_dir)
        .status()
        .unwrap();

    println!("cargo:rustc-link-lib=static=asmfunc");
    println!("cargo:rerun-if-changed=asmfunc.asm");
}

fn build_usb(out_dir: &str, current_dir: &PathBuf) {
    let usb_dir = Path::new(current_dir).join("usb");
    eprintln!("------------ usb_dir {:?}", usb_dir);

    Command::new(Path::new(&usb_dir).join("build.sh"))
        .status()
        .unwrap();

    fs::copy(
        PathBuf::from(&usb_dir).join("libusb.a"),
        Path::new(out_dir).join("libusb.a"),
    )
    .unwrap();

    println!("cargo:rustc-link-lib=static=usb");
    // println!("cargo:rerun-if-changed=asmfunc.asm");
}
