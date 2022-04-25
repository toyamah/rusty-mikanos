use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // let out_dir = env::var("OUT_DIR").unwrap();
    // let current_dir = env::current_dir().unwrap();

    // build_asm(&out_dir, &current_dir);

    // println!("cargo:rustc-link-search=native={}", out_dir);
}

fn build_asm(out_dir: &str, current_dir: &Path) {
    Command::new("nasm")
        .current_dir(current_dir)
        .args(&["-f", "elf64"])
        .arg("-o")
        .arg(Path::new(out_dir).join("syscall.o"))
        .arg("syscall.asm")
        .status()
        .unwrap();

    Command::new("ar")
        .args(&["crs", "libsyscall.a", "syscall.o"])
        .current_dir(out_dir)
        .status()
        .unwrap();

    println!("cargo:rustc-link-lib=static=syscall");
    println!("cargo:rerun-if-changed=syscall.asm");
}
