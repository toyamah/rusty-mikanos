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
