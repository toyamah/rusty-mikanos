enum Error {
    Success,
    Full,
    Empty,
    LastOfCode,
}

impl Error {
    fn is_success(&self) -> bool {
        match self {
            Error::Success => true,
            Error::Full | Error::Empty | Error::LastOfCode => false,
        }
    }

    fn name(&self) -> &str {
        match self {
            Error::Success => "Success",
            Error::Full => "Full",
            Error::Empty => "Empty",
            Error::LastOfCode => "LastOfCode",
        }
    }
}
