use crate::rust_official::cchar::c_char;

pub(crate) mod c_str;
pub(crate) mod cchar;

extern "C" {
    pub(crate) fn strlen(cs: *const c_char) -> usize;
}
