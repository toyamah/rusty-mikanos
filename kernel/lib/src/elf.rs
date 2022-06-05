use crate::error::{Code, Error};
use crate::make_error;
use crate::memory_manager::BitmapMemoryManager;
use crate::paging::{LinearAddress4Level, PageMapEntry};
use core::{cmp, mem};

const EI_NIDENT: usize = 16;

const ET_NONE: Elf64Half = 0;
const ET_REL: Elf64Half = 1;
const ET_EXEC: Elf64Half = 2;
const ET_DYN: Elf64Half = 3;
const ET_CORE: Elf64Half = 4;

type Elf64Addr = usize;
type Elf64Off = u64;
type Elf64Half = u16;
type Elf64Word = u32;
type Elf64Sword = i32;
type Elf64Xword = u64;
type Elf64Sxword = i64;

#[repr(C)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: Elf64Half,
    pub e_machine: Elf64Half,
    pub e_version: Elf64Word,
    pub e_entry: Elf64Addr,
    pub e_phoff: Elf64Off,
    pub e_shoff: Elf64Off,
    pub e_flags: Elf64Word,
    pub e_ehsize: Elf64Half,
    pub e_phentsize: Elf64Half,
    pub e_phnum: Elf64Half,
    pub e_shentsize: Elf64Half,
    pub e_shnum: Elf64Half,
    pub e_shstrndx: Elf64Half,
}

impl Elf64Ehdr {
    pub(crate) unsafe fn from(file_buf: &[u8]) -> Option<&Elf64Ehdr> {
        let header_size = mem::size_of::<Elf64Ehdr>();
        let elf_header = &file_buf[..header_size] as *const _ as *const Elf64Ehdr;
        elf_header.as_ref()
    }

    pub(crate) unsafe fn from_mut(file_buf: &mut [u8]) -> Option<&mut Elf64Ehdr> {
        let header_size = mem::size_of::<Elf64Ehdr>();
        let elf_header = &mut file_buf[..header_size] as *mut _ as *mut Elf64Ehdr;
        elf_header.as_mut()
    }

    pub(crate) fn is_elf(&self) -> bool {
        &self.e_ident[..4] == b"\x7fELF"
    }

    pub unsafe fn get_program_header(&self) -> *const Elf64Phdr {
        let address = self as *const _ as usize;
        (address + self.e_phoff as usize) as *const Elf64Phdr
    }

    pub unsafe fn get_first_load_address(&self) -> usize {
        let phdr = self.get_program_header();
        for i in 0..self.e_phnum {
            let p = phdr.add(i as usize).as_ref().unwrap();
            if p.p_type != PT_LOAD {
                continue;
            }
            return p.p_vaddr;
        }
        0
    }

    pub fn load_elf(
        &mut self,
        cr_3: u64,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<u64, Error> {
        if self.e_type != ET_EXEC {
            return Err(make_error!(Code::InvalidFormat));
        }

        let addr_first = unsafe { self.get_first_load_address() };
        if addr_first < 0xffff_8000_0000_0000 {
            return Err(make_error!(Code::InvalidFormat));
        }

        self.copy_load_segment(cr_3, memory_manager)
    }

    fn copy_load_segment(
        &mut self,
        cr_3: u64,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<u64, Error> {
        let phdr = unsafe { self.get_program_header() };
        let mut last_addr = 0;
        for i in 0..self.e_phnum {
            let p = unsafe { phdr.add(i as usize).as_ref().unwrap() };
            if p.p_type != PT_LOAD {
                continue;
            }

            let dest_addr = LinearAddress4Level::new(p.p_vaddr as u64);
            last_addr = cmp::max(last_addr, p.p_vaddr as u64 + p.p_memsz);
            let num_4kpages: usize = ((p.p_memsz + 4095) / 4096) as usize;
            PageMapEntry::setup_page_maps(dest_addr, num_4kpages, false, cr_3, memory_manager)?;

            let src = unsafe { (self as *mut _ as *mut u8).offset(p.p_offset as isize) };
            let dst = p.p_vaddr as *mut Elf64Addr as *mut u8;
            unsafe {
                src.copy_to_nonoverlapping(dst, p.p_filesz as usize);

                dst.offset(p.p_filesz as isize)
                    .write_bytes(0, (p.p_memsz - p.p_filesz) as usize);
            }
        }
        Ok(last_addr)
    }
}

const PT_NULL: Elf64Word = 0;
const PT_LOAD: Elf64Word = 1;
const PT_DYNAMIC: Elf64Word = 2;
const PT_INTERP: Elf64Word = 3;
const PT_NOTE: Elf64Word = 4;
const PT_SHLIB: Elf64Word = 5;
const PT_PHDR: Elf64Word = 6;
const PT_TLS: Elf64Word = 7;

#[repr(C)]
pub struct Elf64Phdr {
    p_type: Elf64Word,
    p_flags: Elf64Word,
    p_offset: Elf64Off,
    p_vaddr: Elf64Addr,
    p_paddr: Elf64Addr,
    p_filesz: Elf64Xword,
    p_memsz: Elf64Xword,
    p_align: Elf64Xword,
}
