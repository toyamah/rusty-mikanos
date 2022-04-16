use crate::error::{Code, Error};
use crate::make_error;
use crate::memory_manager::{BitmapMemoryManager, FrameID, BYTES_PER_FRAME};
use crate::paging::{LinearAddress4Level, PageMapEntry};
use core::ffi::c_void;
use core::mem;

const EI_NIDENT: usize = 16;

pub const ET_NONE: Elf64Half = 0;
const ET_REL: Elf64Half = 1;
pub const ET_EXEC: Elf64Half = 2;
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

    pub(crate) fn is_elf(&self) -> bool {
        &self.e_ident[..4] == b"\x7fELF"
    }

    pub unsafe fn get_program_header(&self) -> *const Elf64Phdr {
        let address = self as *const _ as usize;
        (address + self.e_phoff) as *const Elf64Phdr
    }

    pub unsafe fn get_first_load_address(&self) -> usize {
        let phdr = self.get_program_header();
        for i in 0..self.e_phnum {
            let p = phdr.add(i as usize);
            if p.p_type == PT_LOAD {
                continue;
            }
            return p.p_vaddr;
        }
        return 0;
    }

    pub fn copy_load_segment(
        &self,
        cr_3: usize,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        let phdr = unsafe { self.get_program_header() };
        for i in 0..self.e_phnum {
            let p = unsafe { phdr.add(i as usize) };
            if p.p_type != PT_LOAD {
                continue;
            }

            let dest_addr = LinearAddress4Level(p.p_vaddr as u64);
            let num_4kpages: usize = ((p.p_memsz + 4095) / 4096) as usize;
            let result =
                PageMapEntry::setup_page_maps(dest_addr, num_4kpages, cr_3, memory_manager);
            if result.is_err() {
                return result;
            }

            let src = unsafe { (self as *const _ as *const u8).offset(p.p_offse as isize) };
            let dst = p.p_vaddr as *const _ as *const u8;
            unsafe {
                memcpy(dst as *mut c_void, src as *mut c_void, p.p_filesz as usize);
                memset(
                    dst.offset(p.p_filesz as isize) as *mut c_void,
                    0,
                    (p.p_memsz - p.p_filesz) as usize,
                );
            }
        }
        Ok(())
    }

    pub fn load_elf(
        &self,
        cr_3: usize,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        if self.e_type != ET_EXEC {
            return Err(make_error!(Code::InvalidFormat));
        }

        let addr_first = unsafe { self.get_first_load_address() };
        if addr_first < 0xffff_8000_0000_0000 {
            return Err(make_error!(Code::InvalidFormat));
        }

        self.copy_load_segment(cr_3, memory_manager)
    }

    pub fn clean_page_map(
        page_map: *mut PageMapEntry,
        page_map_level: i32,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        for i in 0..512 {
            let mut entry = unsafe { page_map.add(i) };
            if entry.present() {
                continue;
            }

            if page_map_level > 1 {
                let result = Self::clean_page_map(entry.pointer(), page - 1, memory_manager);
                if result.is_err() {
                    return result;
                }
            }

            let entry_addr = entry.pointer() as usize;
            let map_frame = FrameID::new(entry_addr / BYTES_PER_FRAME);
            let result = memory_manager.free(map_frame, 1);
            if result.is_err() {
                return result;
            }

            entry.reset();
        }

        Ok(())
    }

    pub fn clean_page_maps(
        addr: LinearAddress4Level,
        cr3: usize,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        let pm4_table = cr3 as *mut _ as *mut PageMapEntry;
        let pdp_table = unsafe { pm4_table.offset(addr.pml4() as isize) }.pointer();
        unsafe { pdp_table.add(addr.pml4() as usize) }.reset();

        let result = Self::clean_page_map(pdp_table, 3, memory_manager);
        if result.is_err() {
            return result;
        }

        let pdp_addr = pdp_table as usize;
        let pdp_frame = FrameID::new(pdp_addr / BYTES_PER_FRAME);
        memory_manager.free(pdp_frame, 1)
    }
}

extern "C" {
    pub fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void;
    pub fn memcpy(dst: *mut c_void, src: *mut c_void, n: usize);
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
struct Elf64Phdr {
    p_type: Elf64Word,
    p_flags: Elf64Word,
    p_offse: Elf64Off,
    p_vaddr: Elf64Addr,
    p_paddr: Elf64Addr,
    p_filesz: Elf64Xword,
    p_memsz: Elf64Xword,
    p_align: Elf64Xword,
}
