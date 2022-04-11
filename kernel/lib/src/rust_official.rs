use crate::rust_official::cchar::c_char;

pub(crate) mod cchar;
pub(crate) mod cstring;

extern "C" {
    pub(crate) fn strlen(cs: *const c_char) -> usize;
}
