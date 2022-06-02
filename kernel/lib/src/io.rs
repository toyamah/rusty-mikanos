use crate::fat::global::{boot_volume_image, boot_volume_image_mut};
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

    pub(crate) fn write(&mut self, buf: &[u8]) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.write(buf, boot_volume_image_mut()),
            FileDescriptor::Terminal(fd) => fd.write(buf),
        }
    }

    pub(crate) fn load(&mut self, buf: &mut [u8], offset: usize) -> usize {
        match self {
            FileDescriptor::Fat(fd) => fd.load(buf, offset, boot_volume_image_mut()),
            FileDescriptor::Terminal(fd) => fd.load(buf, offset),
        }
    }
}
