// uint32_t MakeAddress(uint8_t bus, uint8_t device,
// uint8_t function, uint8_t reg_addr) {
// auto shl = [](uint32_t x, unsigned int bits) {
// return x << bits;
// };
//
// return shl(1, 31)  // enable bit
// | shl(bus, 16)
// | shl(device, 11)
// | shl(function, 8)
// | (reg_addr & 0xfcu);
// }

use core::ops::Shl;

/// make a 32-bit unsigned integer for CONFIG_ADDRESS
fn make_address(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
    let enabled_bit = 1.shl(32);
    enabled_bit | bus.shl(16) | device.shl(11) | function.shl(8) | (reg_addr & 0xfc)
}
