use core::{mem, slice};

pub mod global {
    use crate::fat::Bpb;

    static mut BOOT_VOLUME_IMAGE: Option<&'static Bpb> = None;
    pub fn boot_volume_image() -> &'static Bpb {
        unsafe { BOOT_VOLUME_IMAGE.unwrap() }
    }

    pub fn initialize(volume_image: *const u8) {
        let bpb = unsafe { (volume_image as *const u8 as *const Bpb).as_ref().unwrap() };
        unsafe { BOOT_VOLUME_IMAGE = Some(bpb) };
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Bpb {
    jump_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sector_count: u16,
    num_fats: u8,
    root_entry_count: u16,
    total_sectors_16: u16,
    media: u8,
    fat_size_16: u16,
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    fat_size_32: u32,
    ext_flags: u16,
    fs_version: u16,
    root_cluster: u32,
    fs_info: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    reserved1: u8,
    boot_signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type: [u8; 8],
}

impl Bpb {
    pub fn root_dir_entries(&self) -> &[DirectoryEntry] {
        let size = self.get_entries_per_cluster();
        unsafe {
            let data = self.get_cluster_addr(self.root_cluster as u64);
            slice::from_raw_parts(data.cast(), size)
        }
    }

    fn get_entries_per_cluster(&self) -> usize {
        self.bytes_per_sector as usize / mem::size_of::<DirectoryEntry>()
            * self.sectors_per_cluster as usize
    }

    fn get_cluster_addr(&self, cluster: u64) -> *const u32 {
        let sector_num = self.reserved_sector_count as u64
            + self.num_fats as u64 * self.fat_size_32 as u64
            + (cluster - 2) * self.sectors_per_cluster as u64;

        let offset = (sector_num * self.bytes_per_sector as u64) as usize;
        unsafe { (self as *const _ as *const u32).add(offset) }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Attribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeID = 0x08,
    Directory = 0x10,
    Archive = 0x20,
    LongName = 0x0f,
}

#[repr(C)]
pub struct DirectoryEntry {
    name: [u8; 11],
    pub attr: Attribute,
    ntres: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    first_cluster_high: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_low: u16,
    file_size: u16,
}

impl DirectoryEntry {
    pub fn first_cluster(&self) -> u32 {
        self.first_cluster_low as u32 | (self.first_cluster_high as u32) << 16
    }

    pub fn read_name(&self) -> ([u8; 9], [u8; 4]) {
        let mut base = [0; 9];
        base.copy_from_slice(&self.name[..8]);
        base[8] = 0;

        for i in (0..7).rev() {
            if base[i] == 0x20 {
                break;
            }
            base[i] = 0;
        }

        let mut ext = [0; 4];
        ext.copy_from_slice(&self.name[8..]);
        ext[3] = 0;
        for i in (0..2).rev() {
            if ext[i] == 0x20 {
                break;
            }
            ext[i] = 0;
        }

        (base, ext)
    }
}
