#![allow(dead_code)]

use crate::error::Error;
use crate::printk;
use core::fmt;
use core::fmt::{Display, Formatter};

const CONFIG_ADDRESS: u16 = 0x0cf8;

const CONFIG_DATA: u16 = 0x0cfc;

/// ref: https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_.231
const NON_EXISTENT_DEVICE: u16 = 0xffff;

static mut DEVICES: [Device; 32] = [Device {
    bus: 0,
    device: 0,
    function: 0,
    header_type: 0,
}; 32];

pub fn devices() -> &'static [Device] {
    unsafe { &DEVICES[..NUM_DEVICE] }
}

static mut NUM_DEVICE: usize = 0;

#[derive(Copy, Clone, Debug)]
pub struct Device {
    bus: u8,
    device: u8,
    function: u8,
    header_type: u8,
}

impl Device {
    fn new(bus: u8, device: u8, function: u8, header_type: u8) -> Device {
        Self {
            bus,
            device,
            function,
            header_type,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        read_vendor_id(self.bus, self.device, self.function)
    }

    pub fn class_code(&self) -> u32 {
        read_class_code(self.bus, self.device, self.function)
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}: vend {:04x}, class {:08x}, head {:02x}",
            self.bus,
            self.device,
            self.function,
            self.vendor_id(),
            self.class_code(),
            self.header_type
        )
    }
}

extern "C" {
    fn IoOut32(addr: u16, data: u32);
    fn IoIn32(addr: u16) -> u32;
}

/// make a 32-bit unsigned integer for CONFIG_ADDRESS
///
/// ref: https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_.231
fn make_address(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
    fn shl(x: u8, bits: usize) -> u32 {
        let x = x as u32;
        x << bits
    }
    // this bit enables to read/write data at the made address
    let enabled_bit: u32 = (1 as u32) << 31;
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

fn read_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    write_address(make_address(bus, device, function, 0x00));
    read_data() as u16
}

fn read_device_id(bus: u8, device: u8, function: u8) -> u16 {
    write_address(make_address(bus, device, function, 0x00));
    (read_data() >> 16) as u16
}

fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
    write_address(make_address(bus, device, function, 0x0c));
    (read_data() >> 16) as u8
}

fn read_class_code(bus: u8, device: u8, function: u8) -> u32 {
    write_address(make_address(bus, device, function, 0x08));
    read_data()
}

fn read_bus_number(bus: u8, device: u8, function: u8) -> u32 {
    write_address(make_address(bus, device, function, 0x18));
    read_data()
}

/// ref: https://wiki.osdev.org/PCI#Recursive_Scan
pub fn scan_all_bus() -> Result<(), Error> {
    unsafe {
        NUM_DEVICE = 0;
    }
    let header_type = read_header_type(0, 0, 0);
    if is_single_function_device(header_type) {
        // Single PCI host controller
        return scan_bus(0);
    }

    // Multiple PCI host controllers
    for function in 1..8 as u8 {
        if read_vendor_id(0, 0, function) == NON_EXISTENT_DEVICE {
            continue;
        }
        // If it is a multifunction device,
        // then function 0 will be the PCI host controller responsible for bus 0
        let bus = function;
        scan_bus(bus)?;
    }

    Ok(())
}

/// ref: https://wiki.osdev.org/PCI#Recursive_Scan
fn scan_bus(bus: u8) -> Result<(), Error> {
    for device in 0..32 as u8 {
        if read_vendor_id(bus, device, 0) == NON_EXISTENT_DEVICE {
            continue;
        }
        scan_device(bus, device)?;
    }

    Ok(())
}

/// ref: https://wiki.osdev.org/PCI#Recursive_Scan
fn scan_device(bus: u8, device: u8) -> Result<(), Error> {
    scan_function(bus, device, 0)?;

    if is_single_function_device(read_header_type(bus, device, 0)) {
        return Ok(());
    }

    // It is a multi-function device, so check remaining functions
    for function in 1..8 as u8 {
        if read_vendor_id(bus, device, function) == NON_EXISTENT_DEVICE {
            continue;
        }
        scan_function(bus, device, function)?;
    }

    Ok(())
}

/// ref: https://wiki.osdev.org/PCI#Recursive_Scan
fn scan_function(bus: u8, device: u8, function: u8) -> Result<(), Error> {
    let header_type = read_header_type(bus, device, function);
    add_device(bus, device, function, header_type)?;

    let class_code = read_class_code(bus, device, function);
    let base = (class_code >> 24) & 0xff;
    let sub = (class_code >> 16) & 0xff;

    // if the device is a PCI to PCI bridge
    if base == 0x06 && sub == 0x04 {
        // scan pci devices which are connected with the secondary_bus
        let bus_numbers = read_bus_number(bus, device, function);
        let secondary_bus = (bus_numbers >> 8) & 0xff;
        return scan_bus(secondary_bus as u8);
    }

    Ok(())
}

fn add_device(bus: u8, device: u8, function: u8, header_type: u8) -> Result<(), Error> {
    unsafe {
        if NUM_DEVICE == DEVICES.len() {
            return Err(Error::Full);
        }
        DEVICES[NUM_DEVICE] = Device::new(bus, device, function, header_type);
        NUM_DEVICE += 1;
    }
    Ok(())
}

/// ref: https://wiki.osdev.org/PCI#Multifunction_Devices
fn is_single_function_device(header_type: u8) -> bool {
    header_type & 0x80 == 0
}
