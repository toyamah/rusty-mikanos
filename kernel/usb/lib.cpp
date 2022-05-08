#include "usb/xhci/xhci.hpp"
#include "usb/classdriver/mouse.hpp"
#include "usb/classdriver/keyboard.hpp"
#include "error.hpp"
#include "logger.hpp"

char xhc_buf[sizeof(usb::xhci::Controller)];
usb::xhci::Controller* xhc;

// ref: https://doc.rust-lang.org/nomicon/ffi.html#targeting-callbacks-to-rust-objects
typedef void (*mouse_observer)(uint8_t, int8_t, int8_t);
typedef void (*keyboard_observer)(uint8_t, uint8_t, bool);

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

    void RegisterKeyboardObserver(keyboard_observer keyboard_observer) {
        usb::HIDKeyboardDriver::default_observer = keyboard_observer;
    }

    uint64_t GetCurrentTaskOSStackPointerInRust();
}

__attribute__((no_caller_saved_registers))
extern "C" uint64_t GetCurrentTaskOSStackPointer() {
    auto p = GetCurrentTaskOSStackPointerInRust();

    // this code is needed to work well...
    if (p == 0) {
        while (1) __asm__("hlt");
    }

    return p;
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
