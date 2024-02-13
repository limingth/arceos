#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::boxed::Box;
use axalloc::{global_allocator, GlobalAllocator};
use axhal::mem::{phys_to_virt, PhysAddr, VirtAddr};
use core::{borrow::BorrowMut, cell::RefMut, f32::consts::E, num::NonZeroUsize, ptr::NonNull};
use log::info;
use std::process::Command;
use xhci::{accessor::Mapper, extended_capabilities, ring::trb::event, Registers};

use self::{command_ring::CommandRing, event_ring::EventRing};

mod command_ring;
mod event_ring;

#[derive(Clone, Copy)]
struct MemoryMapper;

struct XhciController<'a> {
    controller: &'a mut dyn XhciBehaviors,
    event_ring: EventRing,
    command_ring: CommandRing,
}

pub(crate) trait XhciBehaviors {
    fn addr(&self) -> usize;
    fn regs(&mut self) -> &mut Registers<MemoryMapper>;
    fn extra_features(&mut self) -> &mut Option<extended_capabilities::List<MemoryMapper>>;
}

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

impl XhciController<'_> {
    pub(crate) fn new(xhci_behavior_impl: &mut dyn XhciBehaviors) -> Result<XhciController, ()> {
        let mut event_ring = EventRing::new(xhci_behavior_impl.borrow_mut().regs());
        let mut command_ring = CommandRing::new(xhci_behavior_impl.borrow_mut().regs());

        Ok(XhciController {
            controller: xhci_behavior_impl,
            event_ring: event_ring,
            command_ring: command_ring,
        })
    }

    pub(crate) fn init(&mut self) {
        let r = self.controller.regs();

        // get owner ship from bios(?)
        if let Some(extended_caps) = self.controller.extra_features() {
            let iter = extended_caps.into_iter();
            for c in iter.filter_map(Result::ok) {
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
        }

        info!("stop");
        r.operational.usbcmd.update_volatile(|u| {
            u.clear_run_stop();
        });
        info!("wait until halt");
        while !r.operational.usbsts.read_volatile().hc_halted() {}
        info!("start reset");
        r.operational.usbcmd.update_volatile(|u| {
            u.set_host_controller_reset();
        });
        info!("wait until reset complete");
        while r.operational.usbcmd.read_volatile().host_controller_reset() {}
        info!("wait until ready");
        while r.operational.usbsts.read_volatile().controller_not_ready() {}

        let n = r
            .capability
            .hcsparams1
            .read_volatile()
            .number_of_device_slots();
        info!("setting num of slots:{}", n);
        registers::handle(|r| {
            r.operational.config.update_volatile(|c| {
                c.set_max_device_slots_enabled(n);
            });
        });

        info!("init event ring...");
        self.event_ring.update_deq_with_xhci(r);
        self.event_ring.init_segtable(r);

        info!("init command ring...");
        self.command_ring.init();

        // TODO:DCBAA:ref from ramen
    }
}
