#include "usb/xhci/xhci.hpp"
#include "usb/classdriver/mouse.hpp"
#include "error.hpp"
#include "logger.hpp"

char xhc_buf[sizeof(usb::xhci::Controller)];
usb::xhci::Controller* xhc;

// ref: https://doc.rust-lang.org/nomicon/ffi.html#targeting-callbacks-to-rust-objects
typedef void (*mouse_observer)(int8_t, int8_t);

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

    void UsbXhciController_configurePort(XhciController* impl) {
        // TODO: use the passed impl variable
        for (int i = 1; i <= xhc->MaxPorts(); ++i) {
            auto port = xhc->PortAt(i);
            Log(kDebug, "Port %d: IsConnected=%d\n", i, port.IsConnected());

            if (port.IsConnected()) {
                if (auto err = ConfigurePort(*xhc, port)) {
                    Log(kError, "failed to configure port: %s at %s:%d\n",
                    err.Name(), err.File(), err.Line());
                    continue;
                }
            }
        }
    }

    int UsbXhciController_ProcessXhcEvent(XhciController* impl) {
        // TODO: use the passed impl variable
        auto error = ProcessEvent(*xhc);
        return error.Cause();
    }

    bool UsbXhciController_PrimaryEventRing_HasFront(XhciController* impl) {
        return xhc->PrimaryEventRing()->HasFront();
    }

    void RegisterMouseObserver(mouse_observer mouse_observer) {
        usb::HIDMouseDriver::default_observer = mouse_observer;
    }
}

// Define to solve the following
// ld.lld: error: undefined symbol: __cxa_pure_virtual
extern "C" void __cxa_pure_virtual() {
  while (1) __asm__("hlt");
}

// libcxx_support.cpp depends on printk function
int printk(const char* format, ...) {
  // noop
}
