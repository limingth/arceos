#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::sync::Arc;
use axhal::mem::phys_to_virt;
use core::{borrow::BorrowMut, num::NonZeroUsize};
use log::info;
use spinlock::SpinNoIrq;
use xhci::{accessor::Mapper, extended_capabilities, ExtendedCapability, Registers};

use crate::host::{dcbaa, exchanger::command_exchanger, scratchpad};

use self::{command_ring::CommandRing, event_ring::EventRing};
use spinning_top::Spinlock;

pub mod command_ring;
pub mod event_ring;

#[derive(Clone, Copy)]
struct MemoryMapper;

struct XhciController<'a> {
    controller: Arc<Spinlock<&'a mut dyn XhciBehaviors>>,
    event_ring: EventRing,
    command_ring: Arc<Spinlock<CommandRing<'a>>>,
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
        let mut command_ring = Arc::new(Spinlock::new(CommandRing::new(
            xhci_behavior_impl.borrow_mut().regs(),
        )));

        Ok(XhciController {
            controller: Arc::new(Spinlock::new(xhci_behavior_impl)),
            event_ring: event_ring,
            command_ring: command_ring,
        })
    }

    pub(crate) fn init(&mut self) {
        let r = self.controller.lock().regs();

        // get owner ship from bios(?)
        if let Some(extended_caps) = self.controller.lock().extra_features() {
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
        r.operational.config.update_volatile(|c| {
            c.set_max_device_slots_enabled(n);
        });

        info!("init event ring...");
        self.event_ring.update_deq_with_xhci(r);
        self.event_ring.init_segtable(r);

        info!("init command ring...");
        self.command_ring.lock().init();

        dcbaa::init(r);
        scratchpad::init_once(r);
        command_exchanger::init(self.command_ring)
        // TODO:修改为多线程，并兼容异步模型
        // MORE TODO: 好吧，修一下error
    }
}
