#include "usb/xhci/xhci.hpp"
#include "error.hpp"
#include "logger.hpp"

char xhc_buf[sizeof(usb::xhci::Controller)];
usb::xhci::Controller* xhc;

extern "C" {
    typedef struct {
        usb::xhci::Controller* controller;
    } XhciController;

    XhciController UsbXhciController(const uint64_t xhc_mmio_base) {
        xhc = new(xhc_buf) usb::xhci::Controller(xhc_mmio_base);
        return {xhc};
    }

    int UsbXhciController_initialize(XhciController* impl) {
        // TODO: use the passed impl variable
        auto error = xhc->Initialize();
        return error.Cause();
    }

    int UsbXhciController_run(XhciController* impl) {
        // TODO: use the passed impl variable
        auto error = xhc->Run();
        return error.Cause();
    }
}

// Define to solve the following
// ld.lld: error: undefined symbol: __cxa_pure_virtual
extern "C" void __cxa_pure_virtual() {
  while (1) __asm__("hlt");
}