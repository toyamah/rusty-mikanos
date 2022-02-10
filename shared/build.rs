extern crate cbindgen;

use std::env;

/// https://github.com/eqrion/cbindgen/blob/master/docs.md
fn main() {
    let config = cbindgen::Config {
        language: cbindgen::Language::C,
        no_includes: true,
        sys_includes: vec!["stdint.h".to_string()],
        pragma_once: true,
        ..Default::default()
    };

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate_with_config(&crate_dir, config)
        .unwrap()
        .write_to_file("shared_header.h");
}
