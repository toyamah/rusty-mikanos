use crate::fat::global::boot_volume_image;
use crate::fat::FatFileDescriptor;
use crate::terminal::TerminalFileDescriptor;

pub(crate) const STD_IN: &str = "@stdin";

pub(crate) enum FileDescriptor {
    Fat(FatFileDescriptor),
    Terminal(TerminalFileDescriptor),
}

impl FileDescriptor {
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.read(buf, boot_volume_image()),
            FileDescriptor::Terminal(fd) => fd.read(buf),
        }
    }
}
