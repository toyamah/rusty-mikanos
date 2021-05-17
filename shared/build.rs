extern crate cbindgen;

use std::env;

/// https://github.com/eqrion/cbindgen/blob/master/docs.md
fn main() {
    let mut config: cbindgen::Config = Default::default();
    config.language = cbindgen::Language::C;
    config.no_includes = true;
    config.sys_includes = vec!["stdint.h".to_string()];
    config.pragma_once = true;

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate_with_config(&crate_dir, config)
        .unwrap()
        .write_to_file("shared_header.h");
}
