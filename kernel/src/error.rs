#![allow(dead_code)]
pub enum Error {
    Full,
    Empty,
    LastOfCode,
}

impl Error {
    pub fn name(&self) -> &'static str {
        match self {
            Error::Full => "Full",
            Error::Empty => "Empty",
            Error::LastOfCode => "LastOfCode",
        }
    }
}
