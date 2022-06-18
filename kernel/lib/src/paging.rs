use crate::asm::global::invalidate_tlb;
use crate::error::Error;
use crate::memory_manager::{BitmapMemoryManager, FrameID, BYTES_PER_FRAME};
use bit_field::BitField;
use core::mem;
use core::ptr::write_bytes;

/// https://wiki.osdev.org/Paging#64-Bit_Paging

/// Number of page directories to be reserved statically
///
/// 1 Gib x PAGE_DIRECTORY_COUNT will be mapped into virtual address.
const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

/// Stands for page map level 4 table
///
/// This has a reference to a PDPTable.
#[repr(align(4096))]
struct PM4Table([u64; 512]);

/// Stands for Directory Pointer Table
///
/// This has 64 references to a PageDirectory.
/// The size is defined as 512 but only PAGE_DIRECTORY_COUNT is used.
#[repr(align(4096))]
struct PDPTable([u64; 512]);

/// PageDirectory has 64 page directories that have 512 page tables
///
/// https://wiki.osdev.org/Paging#Page_Table
#[repr(align(4096))]
struct PageDirectory([[u64; 512]; PAGE_DIRECTORY_COUNT]);

#[repr(transparent)]
pub struct PageMapEntry(u64);

impl PageMapEntry {
    // uint64_t present : 1;
    pub fn present(&self) -> u64 {
        self.0.get_bits(..1) as u64
    }
    pub fn set_present(&mut self, v: u64) {
        self.0.set_bits(..1, v);
    }

    // uint64_t writable : 1;
    pub fn writable(&self) -> bool {
        let v = self.0.get_bits(1..2) as u64;
        1 == v
    }
    pub fn set_writable(&mut self, writable: bool) {
        let value = if writable { 1 } else { 0 };
        self.0.set_bits(1..2, value);
    }

    // uint64_t user : 1;
    pub fn user(&self) -> u64 {
        self.0.get_bits(2..3) as u64
    }
    pub fn set_user(&mut self, v: u64) {
        self.0.set_bits(2..3, v);
    }

    // uint64_t write_through : 1;
    pub fn write_through(&self) -> u64 {
        self.0.get_bits(3..4) as u64
    }

    // uint64_t cache_disable : 1;
    pub fn cache_disable(&self) -> u64 {
        self.0.get_bits(4..5) as u64
    }
    // uint64_t accessed : 1;
    pub fn accessed(&self) -> u64 {
        self.0.get_bits(5..6) as u64
    }
    // uint64_t dirty : 1;
    pub fn dirty(&self) -> u64 {
        self.0.get_bits(6..7) as u64
    }
    // uint64_t huge_page : 1;
    pub fn huge_page(&self) -> u64 {
        self.0.get_bits(7..8) as u64
    }
    // uint64_t global : 1;
    pub fn global(&self) -> u64 {
        self.0.get_bits(8..9) as u64
    }

    // uint64_t : 3;
    // pub fn _a(&self) {
    //     self.0.get_bits(9..12) as u64
    // }

    // uint64_t addr : 40;
    pub fn addr(&self) -> u64 {
        self.0.get_bits(12..52) as u64
    }
    pub fn set_addr(&mut self, addr: u64) {
        self.0.set_bits(12..52, addr);
    }

    // uint64_t : 12;
    // each bit must be the same bit as 47th bit

    pub fn pointer(&self) -> *mut PageMapEntry {
        (self.addr() << 12) as *const u64 as *mut PageMapEntry
    }

    fn set_pointer(&mut self, p: *const PageMapEntry) {
        let p = p as usize as u64;
        let addr = p >> 12;
        self.set_addr(addr);
    }

    pub fn reset(&mut self) {
        self.0 = 0;
    }

    pub fn setup_page_maps(
        addr: LinearAddress4Level,
        num4_kpages: usize,
        writable: bool,
        cr_3: u64,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        let pml4_table = cr_3 as *mut u64 as *mut PageMapEntry;
        let (_, error) =
            Self::setup_page_map(pml4_table, 4, writable, addr, num4_kpages, memory_manager);
        match error {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }

    fn setup_page_map(
        page_map: *mut PageMapEntry,
        page_map_level: i32,
        writable: bool,
        mut addr: LinearAddress4Level,
        mut num_4kpages: usize,
        memory_manager: &mut BitmapMemoryManager,
    ) -> (usize, Option<Error>) {
        while num_4kpages > 0 {
            let entry_index = addr.part(page_map_level);

            let p = unsafe { page_map.add(entry_index as usize) };
            let page_map_ref = unsafe { p.as_mut() }.expect("failed to as mut MapEntry");
            let child_map_result = page_map_ref.set_new_page_map_if_not_present(memory_manager);
            if let Err(e) = child_map_result {
                return (num_4kpages, Some(e));
            }

            let child_map = child_map_result.unwrap();
            page_map_ref.set_user(1);

            if page_map_level == 1 {
                page_map_ref.set_writable(writable);
                num_4kpages -= 1;
            } else {
                page_map_ref.set_writable(true);
                let (num_remain_pages, error) = Self::setup_page_map(
                    child_map,
                    page_map_level - 1,
                    writable,
                    addr,
                    num_4kpages,
                    memory_manager,
                );
                if error.is_some() {
                    return (num_4kpages, error);
                }
                num_4kpages = num_remain_pages;
            }

            if entry_index == 511 {
                break;
            }

            addr.set_part(page_map_level, entry_index + 1);
            for level in (1..=page_map_level - 1).rev() {
                addr.set_part(level, 0);
            }
        }

        (num_4kpages, None)
    }

    fn set_new_page_map_if_not_present(
        &mut self,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<*mut PageMapEntry, Error> {
        if self.present() != 0 {
            return Ok(self.pointer());
        }

        let child_map_result = Self::new_page_map(memory_manager);
        if let Ok(child) = child_map_result {
            self.set_pointer(child);
            self.set_present(1);
        }
        child_map_result
    }

    pub fn new_page_map(
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<*mut PageMapEntry, Error> {
        let frame = memory_manager.allocate(1);
        if let Err(e) = frame {
            return Err(e);
        }
        let frame = frame.unwrap();

        unsafe {
            write_bytes(
                (frame.id() * BYTES_PER_FRAME) as *mut u8,
                0,
                mem::size_of::<u64>() * 512,
            );
        }

        let e = (frame.id() * BYTES_PER_FRAME) as *mut PageMapEntry;
        Ok(e)
    }

    pub fn clean_page_maps(
        addr: LinearAddress4Level,
        cr3: u64,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        let pm4_table = cr3 as *mut u64 as *mut PageMapEntry;
        Self::clean_page_map(pm4_table, 4, addr, memory_manager)
    }

    fn clean_page_map(
        page_map: *mut PageMapEntry,
        page_map_level: i32,
        addr: LinearAddress4Level,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        for i in addr.part(page_map_level) as usize..512 {
            let entry = unsafe { page_map.add(i).as_mut() }.unwrap();
            if entry.present() == 0 {
                continue; // no need to clean this page map entry
            }

            if page_map_level > 1 {
                Self::clean_page_map(entry.pointer(), page_map_level - 1, addr, memory_manager)?;
            }

            if entry.writable() {
                let entry_addr = entry.pointer() as usize;
                let map_frame = FrameID::new(entry_addr / BYTES_PER_FRAME);
                memory_manager.free(map_frame, 1)?;
            }
            entry.reset();
        }

        Ok(())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LinearAddress4Level(u64);

impl LinearAddress4Level {
    pub fn new(v: u64) -> Self {
        Self(v)
    }
    pub fn value(&self) -> u64 {
        self.0
    }

    // uint64_t offset : 12;
    pub fn offset(&self) -> u64 {
        self.0.get_bits(0..12)
    }
    pub fn set_offset(&mut self, v: u64) {
        self.0.set_bits(0..12, v);
    }

    // uint64_t page : 9;
    pub fn page(&self) -> u64 {
        self.0.get_bits(12..21)
    }
    pub fn set_page(&mut self, v: u64) {
        self.0.set_bits(12..21, v);
    }

    // uint64_t dir : 9;
    pub fn dir(&self) -> u64 {
        self.0.get_bits(21..30)
    }
    pub fn set_dir(&mut self, v: u64) {
        self.0.set_bits(21..30, v);
    }

    // uint64_t pdp : 9;
    pub fn pdp(&self) -> u64 {
        self.0.get_bits(30..39)
    }
    pub fn set_pdp(&mut self, v: u64) {
        self.0.set_bits(30..39, v);
    }

    // uint64_t pml4 : 9;
    pub fn pml4(&self) -> u64 {
        self.0.get_bits(39..48)
    }
    pub fn set_pml4(&mut self, v: u64) {
        self.0.set_bits(39..48, v);
    }
    // uint64_t : 16;
    // pub fn a(&self) -> u64 {
    //     self.0.get_bits(48..64)
    // }

    pub fn part(&self, page_map_level: i32) -> u64 {
        match page_map_level {
            0 => self.offset(),
            1 => self.page(),
            2 => self.dir(),
            3 => self.pdp(),
            4 => self.pml4(),
            _ => 0,
        }
    }

    pub fn set_part(&mut self, page_map_level: i32, value: u64) {
        match page_map_level {
            0 => self.set_offset(value),
            1 => self.set_page(value),
            2 => self.set_dir(value),
            3 => self.set_pdp(value),
            4 => self.set_pml4(value),
            _ => {}
        }
    }
}

fn set_page_content(
    table: *mut PageMapEntry,
    part: i32,
    addr: LinearAddress4Level,
    content: *const PageMapEntry,
) {
    if part == 1 {
        let i = addr.part(part) as usize;
        let entry = unsafe { table.add(i).as_mut() }.unwrap();
        entry.set_pointer(content);
        entry.set_writable(true);
        invalidate_tlb(addr.value());
        return;
    }

    let i = addr.part(part) as usize;
    let entry = unsafe { table.add(i).as_mut() }.unwrap();
    set_page_content(entry.pointer(), part - 1, addr, content)
}

pub mod global {
    use super::{
        PDPTable, PM4Table, PageDirectory, PAGE_DIRECTORY_COUNT, PAGE_SIZE_1G, PAGE_SIZE_2M,
    };
    use crate::asm::global::{get_cr0, get_cr3, set_cr0, set_cr3};
    use crate::error::{Code, Error};
    use crate::io::FileDescriptor;
    use crate::libc::memcpy;
    use crate::make_error;
    use crate::memory_manager::global::memory_manager;
    use crate::memory_manager::{FrameID, BYTES_PER_FRAME};
    use crate::paging::{set_page_content, LinearAddress4Level, PageMapEntry};
    use crate::task::global::task_manager;
    use crate::task::FileMapping;
    use core::ffi::c_void;
    use core::slice;

    static mut PML4_TABLE: PM4Table = PM4Table([0; 512]);
    static mut PDP_TABLE: PDPTable = PDPTable([0; 512]);
    static mut PAGE_DIRECTORY: PageDirectory = PageDirectory([[0; 512]; PAGE_DIRECTORY_COUNT]);

    pub fn initialize() {
        setup_identity_page_table();
    }

    /// Set up as virtual addresses = physical addresses
    fn setup_identity_page_table() {
        unsafe {
            PML4_TABLE.0[0] = &PDP_TABLE.0[0] as *const _ as u64 | 0x003;

            for i_pdpt in 0..PAGE_DIRECTORY.0.len() {
                PDP_TABLE.0[i_pdpt] = &PAGE_DIRECTORY.0[i_pdpt] as *const _ as u64 | 0x003;
                for i_pd in 0..512 {
                    PAGE_DIRECTORY.0[i_pdpt][i_pd] =
                        (i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M) | 0x083;
                }
            }

            // set the address of PM4_TABLE to the cr3 register
            set_cr3(&PML4_TABLE.0[0] as *const _ as u64);
            set_cr0(get_cr0() & 0xfffeffff);
        }
    }

    pub(crate) fn reset_cr3() {
        unsafe { set_cr3(&PML4_TABLE.0[0] as *const _ as u64) }
    }

    pub(crate) fn handle_page_fault(error_code: u64, causal_addr: u64) -> Result<(), Error> {
        let task = task_manager().current_task_mut();
        let present = (error_code & 1) == 1;
        let rw = ((error_code >> 1) & 1) == 1;
        let user = ((error_code >> 2) & 1) == 1;

        if present && rw && user {
            return copy_on_page(causal_addr);
        } else if present {
            return Err(make_error!(Code::AlreadyAllocated));
        }

        if task.dpaging_begin <= causal_addr && causal_addr < task.dpaging_end {
            PageMapEntry::setup_page_maps(
                LinearAddress4Level::new(causal_addr),
                1,
                true,
                get_cr3(),
                memory_manager(),
            )
        } else if let Some(fm) = task.find_file_mapping(causal_addr) {
            let fm = fm.clone();
            let fd = task.get_file_mut(fm.fd).unwrap();
            prepare_page_cache(fd, fm, causal_addr)
        } else {
            Err(make_error!(Code::IndexOutOfRange))
        }
    }

    pub fn free_page_map(table: *mut PageMapEntry) -> Result<(), Error> {
        let addr = table as *const _ as usize;
        let frame_id = FrameID::new(addr as usize / BYTES_PER_FRAME);
        memory_manager().free(frame_id, 1)
    }

    pub(crate) fn prepare_page_cache(
        fd: &mut FileDescriptor,
        fm: FileMapping,
        causal_vaddr: u64,
    ) -> Result<(), Error> {
        let mut page_vaddr = LinearAddress4Level::new(causal_vaddr);
        page_vaddr.set_offset(0);
        PageMapEntry::setup_page_maps(page_vaddr, 1, true, get_cr3(), memory_manager())?;

        let file_offset = page_vaddr.value() - fm.vaddr_begin;
        let page_cache =
            unsafe { slice::from_raw_parts_mut(page_vaddr.value() as *mut u64 as *mut u8, 4096) };
        fd.load(page_cache, file_offset as usize);
        Ok(())
    }

    fn copy_on_page(causal_addr: u64) -> Result<(), Error> {
        let p = PageMapEntry::new_page_map(memory_manager())?;
        let aligned_addr = causal_addr & 0xffff_ffff_ffff_f000;
        unsafe { memcpy(p as *mut c_void, aligned_addr as *const c_void, 4096) };
        set_page_content(
            get_cr3() as *mut u64 as *mut PageMapEntry,
            4,
            LinearAddress4Level::new(causal_addr),
            p,
        );
        Ok(())
    }

    pub(crate) fn copy_page_maps(
        dest: *mut PageMapEntry,
        src: *const PageMapEntry,
        part: i32,
        start: i32,
    ) -> Result<(), Error> {
        let dest_at = |i: usize| unsafe { dest.add(i).as_mut() }.unwrap();
        let src_at = |i: usize| unsafe { src.add(i).as_ref() }.unwrap();

        if part == 1 {
            for i in start as usize..512 {
                if src_at(i).present() == 0 {
                    continue;
                }

                let d = dest_at(i);
                d.0 = src_at(i).0;
                d.set_writable(false);
            }
            return Ok(());
        }

        for i in start as usize..512 {
            let s = src_at(i);
            if s.present() == 0 {
                continue;
            }

            let table = PageMapEntry::new_page_map(memory_manager())?;
            let d = dest_at(i);
            d.0 = s.0;
            d.set_pointer(table);
            copy_page_maps(table, s.pointer(), part - 1, 0)?;
        }

        Ok(())
    }
}
