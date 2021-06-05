#include "usb/xhci/xhci.hpp"

extern "C" {
    typedef struct {
        usb::xhci::Controller* controller;
    } XhciController;

    XhciController* UsbXhciController(uint64_t xhc_mmio_base) {
        usb::xhci::Controller xhc{xhc_mmio_base};
        XhciController xchic = {&xhc};
        return &xchic;
    }
}

// Define to solve the following
// ld.lld: error: undefined symbol: __cxa_pure_virtual
extern "C" void __cxa_pure_virtual() {
  while (1) __asm__("hlt");
}