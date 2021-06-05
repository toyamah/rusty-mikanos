extern "C" {
    fn UsbXhciController(xhc_mmio_base: u64) -> *mut XhciControllerImpl;
}

#[derive(Debug)]
enum XhciControllerImpl {}

#[derive(Debug)]
pub struct XhciController {
    c_impl: *mut XhciControllerImpl,
}

impl XhciController {
    pub fn new(xhc_mmio_base: u64) -> Self {
        let c_impl = unsafe { UsbXhciController(xhc_mmio_base) };
        Self { c_impl }
    }
}
