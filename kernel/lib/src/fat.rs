use core::fmt::{Display, Formatter};
use core::mem::size_of;
use core::{cmp, slice};

pub mod global {
    use crate::fat::{next_path_element, Bpb, DirectoryEntry, END_OF_CLUSTER_CHAIN};

    static mut BOOT_VOLUME_IMAGE: Option<&'static Bpb> = None;
    pub fn boot_volume_image() -> &'static Bpb {
        unsafe { BOOT_VOLUME_IMAGE.unwrap() }
    }

    static mut BYTES_PER_CLUSTER: u64 = u64::MAX;
    pub fn bytes_per_cluster() -> u64 {
        unsafe { BYTES_PER_CLUSTER }
    }

    pub fn initialize(volume_image: *const u8) {
        let bpb = unsafe { (volume_image as *const Bpb).as_ref().unwrap() };
        let bytes_per_cluster = bpb.bytes_per_cluster();
        unsafe { BOOT_VOLUME_IMAGE = Some(bpb) };
        unsafe { BYTES_PER_CLUSTER = bytes_per_cluster }
    }

    pub fn find_file(path: &str, mut directory_cluster: u64) -> (Option<&DirectoryEntry>, bool) {
        let mut path = path;
        if path.starts_with('/') {
            directory_cluster = boot_volume_image().root_cluster as u64;
            path = &path[1..];
        } else if directory_cluster == 0 {
            directory_cluster = boot_volume_image().root_cluster as u64;
        }

        let (path_elem, next_path, post_slash) = match next_path_element(path) {
            None => (path, "", false),
            Some(p) => (p.path_before_slash, p.path_after_slash, true),
        };
        let path_last = next_path.is_empty();

        while directory_cluster != END_OF_CLUSTER_CHAIN {
            let dirs =
                boot_volume_image().get_sector_by_cluster::<DirectoryEntry>(directory_cluster);
            for dir in dirs {
                if dir.name[0] == 0x00 {
                    return (None, post_slash);
                } else if !dir.name_is_equal(path_elem) {
                    continue;
                }

                return if dir.is_directory() && !path_last {
                    find_file(next_path, dir.first_cluster() as u64)
                } else {
                    (Some(dir), post_slash)
                };
            }
            directory_cluster = boot_volume_image().next_cluster(directory_cluster);
        }

        (None, post_slash)
    }
}

pub const END_OF_CLUSTER_CHAIN: u64 = 0x0fffffff;

#[repr(packed)]
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

    pub fn get_root_cluster(&self) -> u32 {
        self.root_cluster
    }

    fn get_entries_per_cluster(&self) -> usize {
        self.bytes_per_sector as usize / size_of::<DirectoryEntry>()
            * self.sectors_per_cluster as usize
    }

    fn get_cluster_addr(&self, cluster: u64) -> *const u8 {
        let sector_num = self.reserved_sector_count as u64
            + self.num_fats as u64 * self.fat_size_32 as u64
            + (cluster - 2) * self.sectors_per_cluster as u64;

        let offset = (sector_num * self.bytes_per_sector as u64) as usize;
        unsafe { (self as *const _ as *const u8).add(offset) }
    }

    pub fn next_cluster(&self, cluster: u64) -> u64 {
        let fat = self.get_fat();
        let next = unsafe { fat.add(cluster as usize) };
        unsafe {
            if is_end_of_cluster_chain(*next as u64) {
                END_OF_CLUSTER_CHAIN
            } else {
                (*next).into()
            }
        }
    }

    pub fn get_fat(&self) -> *const u32 {
        let fat_offset = self.reserved_sector_count as usize * self.bytes_per_sector as usize;
        let fat = unsafe { (self as *const _ as *const u8).add(fat_offset) };
        fat as *const u32
    }

    pub fn get_sector_by_cluster<T>(&self, cluster: u64) -> &'static [T] {
        let data = self.get_cluster_addr(cluster);
        let size = self.bytes_per_cluster() as usize / size_of::<T>();
        unsafe { slice::from_raw_parts(data.cast(), size) }
    }

    fn bytes_per_cluster(&self) -> u64 {
        (self.bytes_per_sector as u64) * self.sectors_per_cluster as u64
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Attribute {
    /// Indicates that writes to the file should fail.
    ReadOnly,
    /// Indicates that normal directory listings should not show this file.
    Hidden,
    /// Indicates that this is an operating system file.
    System,
    /// There should only be one “file” on the volume that has this attribute set, /// and that file must be in the root directory.
    /// This name of this file is actually the label for the volume.
    /// DIR_FstClusHI and DIR_FstClusLO must always be 0 for the volume label (no data clusters are allocated to the volume label file).
    VolumeID,
    /// Indicates that this file is actually a container for other files.
    Directory,
    /// This attribute supports backup utilities.
    /// This bit is set by the FAT file system driver when a file is created, renamed, or written to.
    /// Backup utilities may use this attribute to indicate which files on the volume have been modified since the last time that a backup was performed.
    Archive,
    /// Indicates that the “file” is actually part of the long name entry for some other file.
    LongName,
}

pub struct TryFromAttributeError(u8);

impl TryFrom<u8> for Attribute {
    type Error = TryFromAttributeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Attribute::ReadOnly),
            0x02 => Ok(Attribute::Hidden),
            0x04 => Ok(Attribute::System),
            0x08 => Ok(Attribute::VolumeID),
            0x10 => Ok(Attribute::Directory),
            0x20 => Ok(Attribute::Archive),
            0x0f => Ok(Attribute::LongName),
            _ => Err(TryFromAttributeError(value)),
        }
    }
}

impl Display for TryFromAttributeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "unexpected attribute value {}", self.0)
    }
}

/// See 27 page of https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc
#[repr(packed)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    attr: u8,
    ntres: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    first_cluster_high: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_low: u16,
    file_size: u32,
}

impl DirectoryEntry {
    pub fn file_size(&self) -> u32 {
        self.file_size
    }

    pub fn first_cluster(&self) -> u32 {
        self.first_cluster_low as u32 | ((self.first_cluster_high as u32) << 16)
    }

    pub fn attr(&self) -> Option<Attribute> {
        self.attr.try_into().ok()
    }

    pub fn is_directory(&self) -> bool {
        matches!(self.attr.try_into(), Ok(Attribute::Directory))
    }

    /// the directory entry is free (there is no file or directory name in this entry).
    pub fn is_free(&self) -> bool {
        self.name[0] == 0xe5
    }

    /// the directory entry is free (same as for 0xE5),
    /// and there are no allocated directory entries after this one (all of the DIR_Name[0] bytes in all of the entries after this one are also set to 0).
    pub fn is_free_and_no_more_allocated_after_this(&self) -> bool {
        self.name[0] == 0x00
    }

    pub fn basename(&self) -> [u8; 8] {
        let mut base = [0; 8];
        base.copy_from_slice(&self.name[..8]);
        for i in (0..base.len()).rev() {
            if base[i] != 0x20 {
                break;
            }
            base[i] = 0;
        }
        base
    }

    pub fn formatted_name(&self) -> [u8; 12] {
        let mut dest = [0_u8; 12];
        let base = self.basename();
        let ext = self.extension();

        dest[..base.len()].copy_from_slice(&base);

        if ext[0] != 0 {
            let (null_index, _) = dest
                .iter()
                .enumerate()
                .find(|(_, &b)| b == 0)
                .expect("failed to find null terminator");
            dest[null_index] = b'.';

            let first_ext_i = null_index + 1;
            dest[first_ext_i..first_ext_i + ext.len()].copy_from_slice(&ext);
        }

        dest
    }

    pub fn extension(&self) -> [u8; 3] {
        let mut ext = [0; 3];
        ext.copy_from_slice(&self.name[8..]);
        for i in (0..ext.len()).rev() {
            if ext[i] != 0x20 {
                break;
            }
            ext[i] = 0;
        }
        ext
    }

    pub fn name_is_equal(&self, name: &str) -> bool {
        let mut name83: [u8; 11] = [0x20; 11];
        let name = name.as_bytes();

        let mut i = 0;
        let mut i83 = 0;
        loop {
            if i >= name.len() || i83 >= name83.len() {
                break;
            }
            if name[i] == b'.' {
                i83 = 7;
            } else {
                name83[i83] = name[i].to_ascii_uppercase();
            }
            i += 1;
            i83 += 1;
        }

        self.name == name83
    }

    pub fn load_file(&self, buf: &mut [u8], bpb: &Bpb) -> usize {
        fn is_valid_cluster(c: u64) -> bool {
            c != 0 && c != END_OF_CLUSTER_CHAIN
        }

        let mut cluster = self.first_cluster() as u64;
        let buffer_len = buf.len();
        let mut p = buf;

        let mut remain_bytes = buffer_len;
        let bytes_per_cluster = bpb.bytes_per_cluster() as usize;
        while is_valid_cluster(cluster) {
            let copy_bytes = cmp::min(bytes_per_cluster, remain_bytes);
            let sector = bpb.get_sector_by_cluster::<u8>(cluster as u64);
            p[..copy_bytes].copy_from_slice(&sector[..copy_bytes]);

            remain_bytes -= copy_bytes;
            p = &mut p[copy_bytes..];
            cluster = bpb.next_cluster(cluster);
        }

        p.len()
    }
}

#[derive(Eq, PartialEq, Debug)]
struct PathElements<'a> {
    path_before_slash: &'a str,
    path_after_slash: &'a str,
}

impl<'a> PathElements<'a> {
    fn new(path_before_slash: &'a str, path_after_slash: &'a str) -> Self {
        Self {
            path_before_slash,
            path_after_slash,
        }
    }
}

pub(crate) struct FatFileDescriptor {
    // hold a raw pointer to avoid adding lifetime parameters to many structs...
    fat_entry: *const DirectoryEntry,
    rd_off: usize,
    rd_cluster: u64,
    rd_cluster_off: usize,
}

impl FatFileDescriptor {
    pub(crate) fn new(fat_entry: &DirectoryEntry) -> Self {
        Self {
            fat_entry: fat_entry as *const _,
            rd_off: 0,
            rd_cluster: 0,
            rd_cluster_off: 0,
        }
    }

    pub fn read(&mut self, buf: &mut [u8], bpb: &Bpb) -> usize {
        let fat_entry = match unsafe { self.fat_entry.as_ref() } {
            None => return 0,
            Some(f) => f,
        };

        if self.rd_cluster == 0 {
            self.rd_cluster = fat_entry.first_cluster() as u64;
        }

        let len = cmp::min(buf.len(), fat_entry.file_size as usize - self.rd_off);

        let mut total = 0;
        let bytes_per_cluster = bpb.bytes_per_cluster();
        while total < len {
            let sec = bpb.get_sector_by_cluster::<u8>(self.rd_cluster);
            let n = cmp::min(
                len - total,
                bytes_per_cluster as usize - self.rd_cluster_off,
            );
            buf[..n].copy_from_slice(&sec[self.rd_cluster_off..self.rd_cluster_off + n]);
            total += n;

            self.rd_cluster_off += n;
            if self.rd_cluster_off as u64 == bytes_per_cluster {
                self.rd_cluster = bpb.next_cluster(self.rd_cluster);
                self.rd_cluster_off = 0;
            }
        }

        self.rd_off += total;
        total
    }
}

fn next_path_element(path: &str) -> Option<PathElements> {
    path.find('/').map(|first_slash_index| {
        let path_before_slash = &path[..first_slash_index];
        let path_after_slash = &path[first_slash_index + 1..];
        PathElements::new(path_before_slash, path_after_slash)
    })
}

fn is_end_of_cluster_chain(cluster: u64) -> bool {
    cluster >= 0x0ffffff8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_path_element() {
        assert_eq!(next_path_element(""), None);
        assert_eq!(next_path_element("/"), Some(PathElements::new("", "")));

        assert_eq!(
            next_path_element("/abc/def"),
            Some(PathElements::new("", "abc/def"))
        );
        assert_eq!(
            next_path_element("abc/def"),
            Some(PathElements::new("abc", "def"))
        );
        assert_eq!(
            next_path_element("abc/def/ghi"),
            Some(PathElements::new("abc", "def/ghi"))
        );
    }
}
