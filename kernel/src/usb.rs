use lib::error::Code::*;
use lib::error::{Code, Error};
use lib::make_error;
use log::{error, trace};

extern "C" {
    fn UsbXhciController(xhc_mmio_base: u64) -> *mut XhciControllerImpl;
    fn UsbXhciController_initialize(c_impl: *mut XhciControllerImpl) -> i32;
    fn UsbXhciController_run(c_impl: *mut XhciControllerImpl) -> i32;
    fn UsbXhciController_configurePort(c_impl: *mut XhciControllerImpl);
    fn UsbXhciController_ProcessXhcEvent(c_impl: *mut XhciControllerImpl) -> i32;
    fn UsbXhciController_PrimaryEventRing_HasFront(c_impl: *mut XhciControllerImpl) -> bool;

    /// ref: https://doc.rust-lang.org/nomicon/ffi.html#targeting-callbacks-to-rust-objects
    fn RegisterMouseObserver(cb: extern "C" fn(displacement_x: i8, displacement_y: i8));
}

pub fn register_mouse_observer(cb: extern "C" fn(displacement_x: i8, displacement_y: i8)) {
    unsafe { RegisterMouseObserver(cb) };
}

enum XhciControllerImpl {}

pub struct XhciController {
    c_impl: *mut XhciControllerImpl,
}

impl XhciController {
    pub fn new(xhc_mmio_base: u64) -> Self {
        unsafe {
            Self {
                c_impl: UsbXhciController(xhc_mmio_base),
            }
        }
    }

    pub fn initialize(&self) -> Result<(), Error> {
        let error = unsafe { UsbXhciController_initialize(self.c_impl) };
        trace!("XhciController.initialize finished");
        match convert_to_code(error) {
            None => Ok(()),
            Some(code) => Err(make_error!(code)),
        }
    }

    pub fn run(&self) -> Result<(), Error> {
        let error = unsafe { UsbXhciController_run(self.c_impl) };
        trace!("XhciController.run finished");
        match convert_to_code(error) {
            None => Ok(()),
            Some(code) => Err(make_error!(code)),
        }
    }

    pub fn configure_port(&self) {
        unsafe { UsbXhciController_configurePort(self.c_impl) };
        trace!("XchiController.configure_port finished");
    }

    pub fn process_event(&self) -> Result<(), Error> {
        let error = unsafe { UsbXhciController_ProcessXhcEvent(self.c_impl) };
        // trace!("XchiController.process_event finished. code = {}", error);
        match convert_to_code(error) {
            None => Ok(()),
            Some(code) => Err(make_error!(code)),
        }
    }

    pub fn primary_event_ring_has_front(&self) -> bool {
        unsafe { UsbXhciController_PrimaryEventRing_HasFront(self.c_impl) }
    }
}

fn convert_to_code(code: i32) -> Option<Code> {
    if code == 0 {
        // Success
        return None;
    }

    error!("a cpp error occurs. code = {}", code);

    let code = match code {
        1 => Full,
        2 => Empty,
        3 => NoEnoughMemory,
        4 => IndexOutOfRange,
        5 => HostControllerNotHalted,
        6 => InvalidSlotID,
        7 => PortNotConnected,
        8 => InvalidEndpointNumber,
        9 => TransferRingNotSet,
        10 => AlreadyAllocated,
        11 => NotImplemented,
        12 => InvalidDescriptor,
        13 => BufferTooSmall,
        14 => UnknownDevice,
        15 => NoCorrespondingSetupStage,
        16 => TransferFailed,
        17 => InvalidPhase,
        18 => UnknownXHCISpeedID,
        19 => NoWaiter,
        20 => LastOfCode,
        _ => {
            panic!("unexpected code {}", code);
        }
    };

    Some(code)
}
