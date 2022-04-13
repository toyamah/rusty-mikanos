use bit_field::BitField;

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

    // uint64_t writable : 1;
    pub fn writable(&self) -> u64 {
        self.0.get_bits(1..2) as u64
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
    // pub fn _b(&self) {
    //     self.0.get_bits(52..64) as u64
    // }

    fn pointer(&self) -> *const PageMapEntry {
        (self.addr() << 12) as *const _ as *const PageMapEntry
    }

    fn set_pointer(&mut self, p: &PageMapEntry) {
        self.set_addr(p.0 >> 12)
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
