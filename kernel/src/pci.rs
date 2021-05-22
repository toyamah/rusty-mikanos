use core::ops::Shl;

const CONFIG_ADDRESS: u16 = 0x0cf8;

const CONFIG_DATA: u16 = 0x0cfc;

/// make a 32-bit unsigned integer for CONFIG_ADDRESS
fn make_address(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
    fn shl(x: u8, bits: usize) -> u32 {
        (x << bits) as u32
    }
    let enabled_bit: u32 = (1 as u32) << 32;
    enabled_bit | shl(bus, 16) | shl(device, 11) | shl(function, 8) | (reg_addr & 0xfc) as u32
}
