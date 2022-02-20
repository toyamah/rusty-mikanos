use alloc::format;
use alloc::string::String;
use core::slice;

#[repr(packed)]
pub struct RSDP {
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

pub fn initialize(rsdp: &RSDP) {
    rsdp.validate().expect("RDSP is not valid.");
}

impl RSDP {
    pub fn validate(&self) -> Result<(), String> {
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
        let bytes = unsafe { slice::from_raw_parts(self as *const RSDP as *const u8, length) };
        bytes.iter().fold(0u8, |sum, &byte| sum.wrapping_add(byte))
    }
}
