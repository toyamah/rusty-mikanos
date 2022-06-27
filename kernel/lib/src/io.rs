use crate::fat::global::{boot_volume_image, boot_volume_image_mut};
use crate::fat::FatFileDescriptor;
use crate::terminal::file_descriptor::{PipeDescriptor, TerminalFileDescriptor};
use core::fmt::Write;

pub(crate) const STD_IN: usize = 0;
pub(crate) const STD_OUT: usize = 1;
pub(crate) const STD_ERR: usize = 2;

pub(crate) enum FileDescriptor {
    Fat(FatFileDescriptor),
    Terminal(TerminalFileDescriptor),
    Pipe(PipeDescriptor),
}

impl FileDescriptor {
    pub(crate) fn read(&mut self, buf: &mut [u8]) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.read(buf, boot_volume_image()),
            FileDescriptor::Terminal(fd) => fd.read(buf),
            FileDescriptor::Pipe(fd) => fd.read(buf),
        }
    }

    pub(crate) fn write(&mut self, buf: &[u8]) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.write(buf, boot_volume_image_mut()),
            FileDescriptor::Terminal(fd) => fd.write(buf),
            FileDescriptor::Pipe(fd) => fd.write(buf),
        }
    }

    pub(crate) fn load(&mut self, buf: &mut [u8], offset: usize) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.load(buf, offset, boot_volume_image_mut()),
            FileDescriptor::Terminal(fd) => fd.load(buf, offset),
            FileDescriptor::Pipe(fd) => fd.load(buf, offset),
        }
    }

    pub(crate) fn size(&self) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.size(),
            FileDescriptor::Terminal(fd) => fd.size(),
            FileDescriptor::Pipe(fd) => fd.size(),
        }
    }
}

impl Write for FileDescriptor {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}
