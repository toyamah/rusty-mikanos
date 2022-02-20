use alloc::format;
use alloc::string::String;
use core::{mem, slice};

pub mod global {
    use crate::acpi::{DescriptionHeader, Fadt, Rsdp, Xsdt};
    use log::info;

    static mut FADT: Option<&'static Fadt> = None;

    pub fn initialize(rsdp: &'static Rsdp) {
        rsdp.validate().expect("RDSP is not valid.");

        let xsdt = unsafe { (rsdp.xsdt_address as *const Xsdt).as_ref().unwrap() };
        xsdt.header.validate(b"XSDT").expect("XSDT is not valid.");

        let fadt = xsdt
            .entries()
            .find(|&header| {
                header.validate(b"FACP").map(|_| true).unwrap_or_else(|m| {
                    info!("{}", m);
                    false
                })
            })
            .and_then(|entry| unsafe {
                (entry as *const DescriptionHeader as *const Fadt).as_ref()
            })
            .expect("FADT is not found");

        unsafe { FADT = Some(fadt) };
    }
}

#[repr(C, packed)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

impl Rsdp {
    fn validate(&self) -> Result<(), String> {
        if &self.signature != b"RSD PTR " {
            return Err(format!("invalid signature {:?}\n", self.signature));
        }
        if self.revision != 2 {
            return Err(format!("ACPI revision must be 2: {}", self.revision));
        }

        let sum = self.sum_bytes(20);
        if sum != 0 {
            return Err(format!("sum of 20 bytes must be 0: {}\n", sum));
        }
        let sum = self.sum_bytes(36);
        if sum != 0 {
            return Err(format!("sum of 36 bytes must be 0: {}\n", sum));
        }
        Ok(())
    }

    fn sum_bytes(&self, length: usize) -> u8 {
        unsafe { sum_bytes(self, length) }
    }
}

#[repr(C, packed)]
struct Xsdt {
    pub header: DescriptionHeader,
}

impl Xsdt {
    pub fn count(&self) -> usize {
        self.header.length as usize - mem::size_of::<DescriptionHeader>() / mem::size_of::<u64>()
    }

    fn entries(&self) -> impl Iterator<Item = &DescriptionHeader> {
        let entries = unsafe { (&self.header as *const DescriptionHeader).add(1) as *const u64 };
        (0..self.count())
            .map(move |i| unsafe { entries.add(i).read() })
            .filter_map(|x| unsafe { (x as *const DescriptionHeader).as_ref() })
    }
}

#[repr(C, packed)]
struct DescriptionHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub pubcreator_revision: u32,
}

impl DescriptionHeader {
    fn validate(&self, expected_signature: &[u8]) -> Result<(), String> {
        if self.signature != expected_signature {
            return Err(format!("invalid signature: {:?}", self.signature));
        }

        let sum = unsafe { sum_bytes(self, self.length as usize) };
        if sum != 0 {
            let length = self.length; // assign to a value to suppress a clippy error
            return Err(format!("sum of {} bytes must be 0: {}", length, sum));
        }

        Ok(())
    }
}

#[repr(C, packed)]
struct Fadt {
    header: DescriptionHeader,
    pub reserved1: [u8; 76 - mem::size_of::<DescriptionHeader>()],
    pub pm_tmr_blk: u32,
    pub reserved2: [u8; 112 - 80],
    pub flags: u32,
    pub reserved3: [u8; 276 - 116],
}

unsafe fn sum_bytes<T>(data: &T, length: usize) -> u8 {
    let bytes = slice::from_raw_parts(data as *const _ as *const u8, length);
    bytes.iter().fold(0u8, |sum, &byte| sum.wrapping_add(byte))
}
