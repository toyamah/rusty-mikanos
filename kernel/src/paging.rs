use crate::asm::set_cr3;

const PAGE_DIRECTORY_COUNT: usize = 64;

const PAGE_SIZE_4K: u64 = 4096;
const PAGE_SIZE_2M: u64 = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: u64 = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
struct PM4Table([u64; 512]);

#[repr(align(4096))]
struct PDPTable([u64; 512]);

#[repr(align(4096))]
struct PageDirectory([[u64; 512]; PAGE_DIRECTORY_COUNT]);

static mut PML4_TABLE: PM4Table = PM4Table([0; 512]);
static mut PDP_TABLE: PDPTable = PDPTable([0; 512]);
static mut PAGE_DIRECTORY: PageDirectory = PageDirectory([[0; 512]; PAGE_DIRECTORY_COUNT]);

pub fn setup_identity_page_table() {
    unsafe {
        PML4_TABLE.0[0] = &PDP_TABLE.0[0] as *const _ as u64 | 0x003;

        for i_pdpt in 0..PAGE_DIRECTORY.0.len() {
            PDP_TABLE.0[i_pdpt] = &PAGE_DIRECTORY.0[i_pdpt] as *const _ as u64 | 0x003;
            for i_pd in 0..512 {
                PAGE_DIRECTORY.0[i_pdpt][i_pd] =
                    i_pdpt as u64 * PAGE_SIZE_1G + i_pd as u64 * PAGE_SIZE_2M | 0x083;
            }
        }
        set_cr3(&PML4_TABLE.0[0] as *const _ as u64);
    }
}
