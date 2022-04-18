use crate::error::Error;
use crate::memory_manager::{BitmapMemoryManager, BYTES_PER_FRAME};
use bit_field::BitField;
use core::fmt::Write;
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
pub struct PageMapEntry(pub u64);

impl PageMapEntry {
    // uint64_t present : 1;
    pub fn present(&self) -> u64 {
        self.0.get_bits(..1) as u64
    }
    pub fn set_present(&mut self, v: u64) {
        self.0.set_bits(..1, v);
    }

    // uint64_t writable : 1;
    pub fn writable(&self) -> u64 {
        self.0.get_bits(1..2) as u64
    }
    pub fn set_writable(&mut self, v: u64) {
        self.0.set_bits(1..2, v);
    }

    // uint64_t user : 1;
    pub fn user(&self) -> u64 {
        self.0.get_bits(2..3) as u64
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

    pub fn set_new_page_map_if_not_present(
        &mut self,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<*mut PageMapEntry, Error> {
        if self.present() != 0 {
            return Ok(self.pointer());
        }

        let child_map_result = Self::new_page_map(memory_manager);
        if child_map_result.is_ok() {}
        child_map_result.inspect(|&child| {
            self.set_pointer(child);
            self.set_present(1);
        })
        // // Ok(self)
        // Err(make_error!(Code::AlreadyAllocated))
    }

    pub fn new_page_map(
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<*mut PageMapEntry, Error> {
        let frame = memory_manager.allocate(1);
        if frame.is_err() {
            return Err(frame.unwrap_err());
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

    pub fn setup_page_map(
        page_map: *mut PageMapEntry,
        page_map_level: i32,
        mut addr: LinearAddress4Level,
        mut num_4kpages: usize,
        memory_manager: &mut BitmapMemoryManager,
    ) -> (usize, Option<Error>) {
        while num_4kpages > 0 {
            let entry_index = addr.part(page_map_level);

            let p = unsafe { page_map.add(entry_index as usize) };
            let page_map_ref = unsafe { p.as_mut() }.expect("failed to as mut MapEntry");
            let child_map_result = page_map_ref.set_new_page_map_if_not_present(memory_manager);

            if child_map_result.is_err() {
                return (num_4kpages, Some(child_map_result.unwrap_err()));
            }
            let child_map = child_map_result.unwrap();
            unsafe { page_map_ref.set_writable(1) };

            if page_map_level == 1 {
                num_4kpages -= 1;
            } else {
                let (num_remain_pages, error) = Self::setup_page_map(
                    child_map,
                    page_map_level - 1,
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

    pub fn setup_page_maps(
        addr: LinearAddress4Level,
        num4_kpages: usize,
        cr_3: u64,
        memory_manager: &mut BitmapMemoryManager,
    ) -> Result<(), Error> {
        let pml4_table = cr_3 as *mut u64 as *mut PageMapEntry;
        let (_, error) = Self::setup_page_map(pml4_table, 4, addr, num4_kpages, memory_manager);
        match error {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct LinearAddress4Level(pub u64);

impl LinearAddress4Level {
    pub fn new(v: u64) -> Self {
        Self(v)
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

pub mod global {
    use super::{
        PDPTable, PM4Table, PageDirectory, PAGE_DIRECTORY_COUNT, PAGE_SIZE_1G, PAGE_SIZE_2M,
    };
    use crate::asm::global::set_cr3;

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
        }
    }
}
