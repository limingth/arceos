use log::debug;
use log::error;
use xhci::ExtendedCapability;

use crate::host::structures::extended_capabilities;
use crate::host::structures::registers;
use crate::host::structures::roothub;
use crate::host::structures::scratchpad;
use crate::host::structures::xhci_command_manager;
use crate::host::structures::xhci_event_manager;
use crate::host::structures::xhci_slot_manager;

use super::XHCI_CONFIG_MAX_EVENTS_PER_INTR;

use super::ARM_IRQ_PCIE_HOST_INTA;

pub(crate) fn init(mmio_base: usize) {
    unsafe {
        registers::init(mmio_base);
        extended_capabilities::init(mmio_base);
    };

    debug!("resetting xhci controller");
    reset_xhci_controller();

    xhci_slot_manager::new();
    xhci_event_manager::new();
    xhci_command_manager::new();
    scratchpad::new();
    scratchpad::assign_scratchpad_into_dcbaa();
    roothub::new();

    axhal::irq::register_handler(ARM_IRQ_PCIE_HOST_INTA, interrupt_handler);
    registers::handle(|r| {
        r.operational.usbcmd.update_volatile(|r| {
            r.interrupter_enable();
            r.set_run_stop();
        })
    });

    debug!(
        "init completed!, coltroller state:{:?}",
        registers::handle(|r| r.operational.usbsts.read_volatile())
    );
}

pub(crate) fn interrupt_handler() {
    debug!("interrupt!");
    registers::handle(|r| {
        r.operational.usbsts.update_volatile(|sts| {
            sts.clear_event_interrupt();
        });

        r.interrupter_register_set
            .interrupter_mut(0)
            .iman
            .update_volatile(|iman| {
                iman.clear_interrupt_pending();
            });

        if r.operational.usbsts.read_volatile().hc_halted() {
            error!("HC halted");
            return;
        }

        for tries in 0..XHCI_CONFIG_MAX_EVENTS_PER_INTR {
            if xhci_event_manager::handle_event().is_ok() {}
        }
    })
}

pub(crate) fn reset_xhci_controller() {
    registers::handle(|r| {
        debug!("stop");
        r.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });

        debug!("wait until halt");
        while !r.operational.usbsts.read_volatile().hc_halted() {}
        debug!("halted");

        debug!("HCRST!");
        r.operational.usbcmd.update_volatile(|c| {
            c.set_host_controller_reset();
        });

        while r.operational.usbcmd.read_volatile().host_controller_reset()
            || r.operational.usbsts.read_volatile().controller_not_ready()
        {}

        debug!("get bios ownership");
        for c in extended_capabilities::iter()
            .unwrap()
            .filter_map(Result::ok)
        {
            if let ExtendedCapability::UsbLegacySupport(mut u) = c {
                let l = &mut u.usblegsup;
                l.update_volatile(|s| {
                    s.set_hc_os_owned_semaphore();
                });

                while l.read_volatile().hc_bios_owned_semaphore()
                    || !l.read_volatile().hc_os_owned_semaphore()
                {}
            }
        }

        debug!("Reset xHCI Controller Globally");
    });
}
