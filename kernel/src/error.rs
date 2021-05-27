pub enum Error {
    Full,
    Empty,
    LastOfCode,
}

impl Error {
    pub fn name(&self) -> &str {
        match self {
            Error::Full => "Full",
            Error::Empty => "Empty",
            Error::LastOfCode => "LastOfCode",
        }
    }
}
