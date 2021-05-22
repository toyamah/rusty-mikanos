const CONFIG_ADDRESS: u16 = 0x0cf8;

const CONFIG_DATA: u16 = 0x0cfc;

extern "C" {
    fn IoOut32(addr: u16, data: u32);
    fn IoIn32(addr: u16) -> u32;
}

/// make a 32-bit unsigned integer for CONFIG_ADDRESS
fn make_address(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
    fn shl(x: u8, bits: usize) -> u32 {
        (x << bits) as u32
    }
    let enabled_bit: u32 = (1 as u32) << 32;
    enabled_bit | shl(bus, 16) | shl(device, 11) | shl(function, 8) | (reg_addr & 0xfc) as u32
}

fn write_address(address: u32) {
    unsafe { IoOut32(CONFIG_ADDRESS, address) }
}

fn write_data(value: u32) {
    unsafe {
        IoOut32(CONFIG_DATA, value);
    }
}

fn read_data() -> u32 {
    unsafe { IoIn32(CONFIG_DATA) }
}

fn read_vendor_id(bus: u8, device: u8, function: u8) -> u32 {
    write_address(make_address(bus, device, function, 0x00));
    read_data() & 0xffff
}
