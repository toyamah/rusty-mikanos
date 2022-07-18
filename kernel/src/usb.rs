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
    fn RegisterMouseObserver(
        cb: extern "C" fn(buttons: u8, displacement_x: i8, displacement_y: i8),
    );

    fn RegisterKeyboardObserver(cb: extern "C" fn(modifier: u8, keycode: u8, press: bool));
}

pub mod global {
    use super::XhciController;
    use bit_field::BitField;
    use lib::error::Error;
    use lib::pci::Device;
    use lib::{interrupt, pci};
    use spin::{Lazy, Mutex};

    static XHCI_CONTROLLER: Lazy<Mutex<XhciController>> = Lazy::new(|| {
        let xhc_device = pci::find_xhc_device().expect("no xHC has been found");
        enable_to_interrupt_for_xhc(xhc_device).unwrap();

        let xhc_bar = pci::read_bar(xhc_device, 0).expect("cannot read base address#0");
        let xhc_mmio_base = xhc_bar & !(0x0f_u64);
        // debug!("xHC mmio_base = {:08x}", xhc_mmio_base);

        let controller = XhciController::new(xhc_mmio_base);
        if xhc_device.is_intel_device() {
            xhc_device.switch_ehci_to_xhci();
        }

        controller.initialize().unwrap();
        controller.run().unwrap();
        controller.configure_port();
        Mutex::new(controller)
    });

    pub fn xhci_controller() -> &'static Mutex<XhciController> {
        // execute lazy initialization
        &*XHCI_CONTROLLER
    }

    pub fn initialize() {
        let _ = XHCI_CONTROLLER.lock();
    }

    fn enable_to_interrupt_for_xhc(xhc_device: &Device) -> Result<(), Error> {
        // bsp is bootstrap processor which is the only core running when the power is turned on.
        let bsp_local_apic_id_addr = 0xfee00020 as *const u32;
        let bsp_local_apic_id = unsafe { (*bsp_local_apic_id_addr).get_bits(24..=31) as u8 };

        pci::configure_msi_fixed_destination(
            xhc_device,
            bsp_local_apic_id,
            pci::MsiTriggerMode::Level,
            pci::MsiDeliveryMode::Fixed,
            interrupt::InterruptVectorNumber::XHCI as u8,
            0,
        )
    }
}

pub fn register_mouse_observer(
    cb: extern "C" fn(buttons: u8, displacement_x: i8, displacement_y: i8),
) {
    unsafe { RegisterMouseObserver(cb) };
}

pub fn register_keyboard_observer(cb: extern "C" fn(modifier: u8, keycode: u8, press: bool)) {
    unsafe { RegisterKeyboardObserver(cb) };
}

enum XhciControllerImpl {}

pub struct XhciController {
    c_impl: *mut XhciControllerImpl,
}

unsafe impl Send for XhciController {}

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

    pub fn process_events(&self) {
        while self.primary_event_ring_has_front() {
            if let Err(code) = self.process_event() {
                error!("Error while ProcessEvent: {}", code)
            }
        }
    }

    fn process_event(&self) -> Result<(), Error> {
        let error = unsafe { UsbXhciController_ProcessXhcEvent(self.c_impl) };
        // trace!("XchiController.process_event finished. code = {}", error);
        match convert_to_code(error) {
            None => Ok(()),
            Some(code) => Err(make_error!(code)),
        }
    }

    fn primary_event_ring_has_front(&self) -> bool {
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
        20 => NoSuchTask,
        21 => InvalidFormat,
        22 => FrameTooSmall,
        23 => InvalidFile,
        24 => IsDirectory,
        25 => NoSuchEntry,
        26 => LastOfCode,
        _ => {
            panic!("unexpected code {}", code);
        }
    };

    Some(code)
}
