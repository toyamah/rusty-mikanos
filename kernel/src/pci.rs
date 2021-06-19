#![allow(dead_code)]

use crate::error::{Code, Error};
use crate::make_error;
use bit_field::BitField;
use core::fmt;
use core::fmt::{Display, Formatter};
use log::debug;

const CONFIG_ADDRESS: u16 = 0x0cf8;

const CONFIG_DATA: u16 = 0x0cfc;

/// ref: https://wiki.osdev.org/PCI#Configuration_Space_Access_Mechanism_.231
const NON_EXISTENT_DEVICE: u16 = 0xffff;

static mut DEVICES: [Device; 32] = [Device {
    bus: 0,
    device: 0,
    function: 0,
    header_type: 0,
    class_code: ClassCode {
        base: 0,
        sub: 0,
        interface: 0,
    },
}; 32];

pub fn devices() -> &'static [Device] {
    unsafe { &DEVICES[..NUM_DEVICE] }
}

pub fn find_xhc_device<'a>() -> Option<&'a Device> {
    devices()
        .iter()
        .find(|d| d.is_xhc() && d.is_intel_device())
        .or_else(|| devices().iter().find(|d| d.is_xhc()))
}

static mut NUM_DEVICE: usize = 0;

#[derive(Copy, Clone, Debug)]
pub struct Device {
    bus: u8,
    device: u8,
    function: u8,
    header_type: u8,
    class_code: ClassCode,
}

impl Device {
    fn new(bus: u8, device: u8, function: u8, header_type: u8, class_code: ClassCode) -> Device {
        Self {
            bus,
            device,
            function,
            header_type,
            class_code,
        }
    }

    fn vendor_id(&self) -> u16 {
        read_vendor_id(self.bus, self.device, self.function)
    }

    pub fn is_xhc(&self) -> bool {
        self.class_code.is_match_all(0x0c, 0x03, 0x30)
    }

    /// ref: https://devicehunt.com/view/type/pci/vendor/8086
    pub fn is_intel_device(&self) -> bool {
        self.vendor_id() == 0x8086
    }

    pub fn switch_ehci_to_xhci(&self) {
        let intel_ehc_exist = devices()
            .iter()
            .find(|device| device.is_intel_ehc())
            .is_some();

        if !intel_ehc_exist {
            return;
        }

        let superspeed_ports = read_conf_reg(self, 0xdc); // USB3PRM
        write_conf_reg(self, 0xd8, superspeed_ports); // USB3_PSSEN
        let ehci_to_xhci_ports = read_conf_reg(self, 0xd4); // XUSB2PRM
        write_conf_reg(self, 0xd0, ehci_to_xhci_ports); // XUSB2PR
        debug!(
            "switch_ehci_to_xhci: SS = {:02}, xHCI = {:02x}\n",
            superspeed_ports, ehci_to_xhci_ports
        );
    }

    fn is_intel_ehc(&self) -> bool {
        self.class_code.is_match_all(0x0c, 0x03, 0x20) && self.is_intel_device()
    }
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}: vend {:04x}, class {}, head {:02x}",
            self.bus,
            self.device,
            self.function,
            self.vendor_id(),
            self.class_code,
            self.header_type
        )
    }
}

#[derive(Copy, Clone, Debug)]
struct ClassCode {
    base: u8,
    sub: u8,
    interface: u8,
}

impl ClassCode {
    fn new(base: u8, sub: u8, interface: u8) -> Self {
        Self {
            base,
            sub,
            interface,
        }
    }

    fn is_match_base(&self, base: u8) -> bool {
        base == self.base
    }

    fn is_match_base_sub(&self, base: u8, sub: u8) -> bool {
        self.is_match_base(base) && sub == self.sub
    }

    fn is_match_all(&self, base: u8, sub: u8, interface: u8) -> bool {
        self.is_match_base_sub(base, sub) && interface == self.interface
    }
}

impl From<u32> for ClassCode {
    fn from(reg: u32) -> Self {
        let base = (reg >> 24) as u8;
        let sub = (reg >> 16) as u8;
        let interface = (reg >> 8) as u8;
        ClassCode::new(base, sub, interface)
    }
}

impl Display for ClassCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let value =
            (self.base as u32) << 24 | (self.sub as u32) << 16 | (self.interface as u32) << 8;
        write!(f, "{:08x}", value)
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

fn read_class_code(bus: u8, device: u8, function: u8) -> ClassCode {
    write_address(make_address(bus, device, function, 0x08));
    let reg = read_data();
    ClassCode::from(reg)
}

fn read_bus_number(bus: u8, device: u8, function: u8) -> u32 {
    write_address(make_address(bus, device, function, 0x18));
    read_data()
}

fn read_conf_reg(device: &Device, reg_addr: u8) -> u32 {
    let address = make_address(device.bus, device.device, device.function, reg_addr);
    write_address(address);
    read_data()
}

fn write_conf_reg(device: &Device, reg_addr: u8, value: u32) {
    let address = make_address(device.bus, device.device, device.function, reg_addr);
    write_address(address);
    write_data(value);
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
    let class_code = read_class_code(bus, device, function);
    let header_type = read_header_type(bus, device, function);
    add_device(bus, device, function, header_type, class_code)?;

    // if the device is a PCI to PCI bridge
    if class_code.is_match_base_sub(0x06, 0x04) {
        // scan pci devices which are connected with the secondary_bus
        let bus_numbers = read_bus_number(bus, device, function);
        let secondary_bus = (bus_numbers >> 8) & 0xff;
        return scan_bus(secondary_bus as u8);
    }

    Ok(())
}

fn add_device(
    bus: u8,
    device: u8,
    function: u8,
    header_type: u8,
    class_code: ClassCode,
) -> Result<(), Error> {
    unsafe {
        if NUM_DEVICE == DEVICES.len() {
            return Err(make_error!(Code::Full));
        }
        DEVICES[NUM_DEVICE] = Device::new(bus, device, function, header_type, class_code);
        NUM_DEVICE += 1;
    }
    Ok(())
}

/// ref: https://wiki.osdev.org/PCI#Multifunction_Devices
fn is_single_function_device(header_type: u8) -> bool {
    header_type & 0x80 == 0
}

pub fn read_bar(device: &Device, bar_index: usize) -> Result<u64, Error> {
    if bar_index >= 6 {
        return Err(make_error!(Code::IndexOutOfRange));
    }

    let addr = calc_bar_address(bar_index);
    let bar = read_conf_reg(device, addr) as u64;

    // 32 bit address
    if bar & 4 == 0 {
        return Ok(bar);
    }

    // 64 bit address
    if bar_index >= 5 {
        return Err(make_error!(Code::IndexOutOfRange));
    }

    let bar_upper = read_conf_reg(device, addr + 4) as u64;
    return Ok(bar | bar_upper << 32);
}

fn calc_bar_address(bar_index: usize) -> u8 {
    (0x10 + 4 * bar_index) as u8
}

#[repr(transparent)]
pub struct CapabilityHeader(u32);

impl CapabilityHeader {
    // cap_id : 8;
    // next_ptr : 8;
    // cap : 16;
    fn cap_id(&self) -> u8 {
        self.0.get_bits(..8) as u8
    }

    fn next_ptr(&self) -> u8 {
        self.0.get_bits(8..16) as u8
    }

    fn cap(&self) -> u16 {
        self.0.get_bits(16..) as u16
    }
}

impl From<u32> for CapabilityHeader {
    fn from(value: u32) -> Self {
        Self { 0: value }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct MsiCapabilityHeader(u32);

impl MsiCapabilityHeader {
    // cap_id : 8;
    // next_ptr : 8;
    // msi_enable : 1;
    // multi_msg_capable : 3;
    // multi_msg_enable : 3;
    // addr_64_capable : 1;
    // per_vector_mask_capable : 1;
    fn cap_id(&self) -> u8 {
        self.0.get_bits(..8) as u8
    }
    fn next_ptr(&self) -> u8 {
        self.0.get_bits(8..16) as u8
    }
    fn msi_enable(&self) -> bool {
        self.0.get_bit(16)
    }
    fn set_msi_enable(&mut self, enable: bool) {
        self.0.set_bit(16, enable);
    }
    fn multi_msg_capable(&self) -> u8 {
        self.0.get_bits(17..20) as u8
    }
    fn set_multi_msg_enable(&mut self, enable: u8) {
        self.0.set_bits(17..20, enable as u32);
    }
    fn multi_msg_enable(&self) -> u8 {
        self.0.get_bits(20..23) as u8
    }
    fn addr_64_capable(&self) -> bool {
        self.0.get_bit(23)
    }
    fn per_vector_mask_capable(&self) -> bool {
        self.0.get_bit(24)
    }
}

impl From<u32> for MsiCapabilityHeader {
    fn from(value: u32) -> Self {
        Self { 0: value }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MsiCapability {
    header: MsiCapabilityHeader,
    msg_addr: u32,
    msg_upper_addr: u32,
    msg_data: u32,
    mask_bits: u32,
    pending_bits: u32,
}

const CAPABILITY_MSI: u8 = 0x05;
const CAPABILITY_MSIX: u8 = 0x11;

pub fn read_capability_header(dev: &Device, addr: u8) -> CapabilityHeader {
    let header_data = read_conf_reg(dev, addr);
    CapabilityHeader::from(header_data)
}
pub fn configure_msi(
    dev: &Device,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u32,
) -> Result<(), Error> {
    let mut cap_addr = (read_conf_reg(dev, 0x34) & 0xff) as u8;
    let mut msi_cap_addr: u8 = 0;
    let mut msix_cap_addr: u8 = 0;
    while cap_addr != 0 {
        let header = read_capability_header(dev, cap_addr);
        if header.cap_id() == CAPABILITY_MSI {
            msi_cap_addr = cap_addr;
        } else if header.cap_id() == CAPABILITY_MSIX {
            msix_cap_addr = cap_addr;
        }
        cap_addr = header.next_ptr();
    }

    if msi_cap_addr != 0 {
        configure_msi_register(dev, msi_cap_addr, msg_addr, msg_data, num_vector_exponent)
    } else if msix_cap_addr != 0 {
        configure_msix_register(dev, msix_cap_addr, msg_addr, msg_data, num_vector_exponent)
    } else {
        Err(make_error!(Code::NoPCIMSI))
    }
}

#[derive(Eq, PartialEq)]
pub enum MsiTriggerMode {
    Edge = 0,
    Level = 1,
}

pub enum MsiDeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
}

pub fn configure_msi_fixed_destination(
    dev: &Device,
    apic_id: u8,
    trigger_mode: MsiTriggerMode,
    delivery_mode: MsiDeliveryMode,
    vector: u8,
    num_vector_exponent: u32,
) -> Result<(), Error> {
    let apic_id = apic_id as u32;
    let msg_addr: u32 = 0xfee00000 | (apic_id << 12);

    let delivery_mode = delivery_mode as u32;
    let mut msg_data = (delivery_mode << 8) | vector as u32;
    if trigger_mode == MsiTriggerMode::Level {
        msg_data |= 0xc000;
    }

    configure_msi(dev, msg_addr, msg_data, num_vector_exponent)
}

fn read_msi_capability(dev: &Device, cap_addr: u8) -> MsiCapability {
    let header_data = read_conf_reg(dev, cap_addr);
    let header = MsiCapabilityHeader::from(header_data);

    let msg_addr = read_conf_reg(dev, cap_addr + 4);

    let mut msg_upper_addr = 0;
    let mut msg_data_addr = cap_addr + 8;
    if header.addr_64_capable() {
        msg_upper_addr = read_conf_reg(dev, cap_addr + 8);
        msg_data_addr = cap_addr + 12;
    }

    let msg_data = read_conf_reg(dev, msg_data_addr);

    let mut mask_bits = 0;
    let mut pending_bits = 0;
    if header.per_vector_mask_capable() {
        mask_bits = read_conf_reg(dev, msg_data_addr + 4);
        pending_bits = read_conf_reg(dev, msg_data_addr + 8);
    }

    MsiCapability {
        header,
        msg_addr,
        msg_upper_addr,
        msg_data,
        mask_bits,
        pending_bits,
    }
}

fn write_msi_capability(dev: &Device, cap_addr: u8, msi_cap: &MsiCapability) {
    write_conf_reg(dev, cap_addr, msi_cap.header.0);
    write_conf_reg(dev, cap_addr + 4, msi_cap.msg_addr);

    let mut msg_data_addr = cap_addr + 8;
    if msi_cap.header.addr_64_capable() {
        write_conf_reg(dev, cap_addr + 8, msi_cap.msg_upper_addr);
        msg_data_addr = cap_addr + 12;
    }

    write_conf_reg(dev, msg_data_addr, msi_cap.msg_data);

    if msi_cap.header.per_vector_mask_capable() {
        write_conf_reg(dev, msg_data_addr + 4, msi_cap.mask_bits);
        write_conf_reg(dev, msg_data_addr + 8, msi_cap.pending_bits);
    }
}

fn configure_msi_register(
    dev: &Device,
    cap_addr: u8,
    msg_addr: u32,
    msg_data: u32,
    num_vector_exponent: u32,
) -> Result<(), Error> {
    let mut msi_cap = read_msi_capability(dev, cap_addr);

    if msi_cap.header.multi_msg_capable() <= num_vector_exponent as u8 {
        msi_cap
            .header
            .set_multi_msg_enable(msi_cap.header.multi_msg_capable());
    } else {
        msi_cap
            .header
            .set_multi_msg_enable(num_vector_exponent as u8);
    }

    msi_cap.header.set_msi_enable(true);
    msi_cap.msg_addr = msg_addr;
    msi_cap.msg_data = msg_data;
    write_msi_capability(dev, cap_addr, &msi_cap);
    Ok(())
}

fn configure_msix_register(
    _dev: &Device,
    _cap_addr: u8,
    _msg_addr: u32,
    _msg_data: u32,
    _num_vector_exponent: u32,
) -> Result<(), Error> {
    Err(make_error!(Code::NotImplemented))
}
