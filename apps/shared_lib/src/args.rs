use crate::c_char;
use crate::rust_official::cstr::CStr;

pub struct Args {
    argc: i32,
    argv: *const *const c_char,
}

impl Args {
    pub fn new(argc: i32, argv: *const *const c_char) -> Self {
        Self { argc, argv }
    }

    pub fn get(&self, index: usize) -> &str {
        let ptr = unsafe { *self.argv.add(index as usize) };
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let bytes = c_str.to_bytes();
        core::str::from_utf8(bytes).expect("could not convert to str")
    }

    pub fn len(&self) -> usize {
        self.argc as usize
    }
}
