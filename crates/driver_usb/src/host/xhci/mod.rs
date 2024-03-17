#[cfg(feature = "vl805")]
pub mod vl805;
use axhal::mem::phys_to_virt;
use core::num::NonZeroUsize;
use log::{debug, info};
use xhci::{
    accessor::Mapper,
    extended_capabilities::debug::EventRingDequeuePointer,
    registers::operational::{ConfigureRegister, DeviceNotificationControl},
    ExtendedCapability,
};

use aarch64_cpu::asm::barrier;

use crate::host::structures::{extended_capabilities, xhci_slotmanager};

use super::{
    multitask::{self, task::Task},
    port,
    structures::registers,
};

#[derive(Clone, Copy)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

pub(crate) fn init(mmio_base: usize) {
    unsafe {
        registers::init(mmio_base);
        extended_capabilities::init(mmio_base);
    };

    debug!("resetting xhci controller");
    reset_xhci_controller();

    xhci_slotmanager::new();
}

fn reset_xhci_controller() {
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
        for c in extended_capabilities::iter().filter_map(Result::ok) {
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
