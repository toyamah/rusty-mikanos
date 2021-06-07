use log::{debug, error, info};

extern "C" {
    fn UsbXhciController(xhc_mmio_base: u64) -> *mut XhciControllerImpl;
    fn UsbXhciController_initialize(c_impl: *mut XhciControllerImpl) -> i32;
    fn UsbXhciController_run(c_impl: *mut XhciControllerImpl) -> i32;

}

enum XhciControllerImpl {}

pub struct XhciController {
    c_impl: *mut XhciControllerImpl,
}

impl XhciController {
    pub fn new(xhc_mmio_base: u64) -> Self {
        unsafe {
            Self {
                c_impl: UsbXhciController(xhc_mmio_base)
            }
        }
    }

    pub fn initialize(&self) {
        let error = unsafe { UsbXhciController_initialize(self.c_impl) };
        debug!("Rust initialize end {}", error);
    }

    pub fn run(&self) {
        let error = unsafe { UsbXhciController_run(self.c_impl) };
        debug!("Rust run end {}", error);
    }
}
