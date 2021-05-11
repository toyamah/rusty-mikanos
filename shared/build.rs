extern crate cbindgen;

use std::env;
use cbindgen::RenameRule;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config: cbindgen::Config = Default::default();
    config.language = cbindgen::Language::C;
    config.no_includes = true;
    config.enumeration.rename_variants = RenameRule::CamelCase;
    cbindgen::generate_with_config(&crate_dir, config)
        .unwrap()
        .write_to_file("shared_header.h");
}