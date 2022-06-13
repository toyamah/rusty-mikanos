#![allow(dead_code)]

use core::fmt;
use core::fmt::Formatter;

#[derive(Debug)]
pub struct Error {
    pub code: Code,
    file: &'static str,
    line: u32,
}

impl Error {
    pub fn new(code: Code, file: &'static str, line: u32) -> Self {
        Self { code, file, line }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "code: {}, file = {}, line = {}",
            self.code, self.file, self.line
        )
    }
}

#[derive(Debug)]
pub enum Code {
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotID,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
    NoPCIMSI,
    NoSuchTask,
    InvalidFormat,
    FrameTooSmall,
    InvalidFile,
    IsDirectory,
    NoSuchEntry,
    FreeTypeError,
    LastOfCode,
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[macro_export]
macro_rules! make_error {
    ($x:expr) => {{
        Error::new(($x), file!(), line!())
    }};
}
