#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::sync::Arc;
use axhal::mem::phys_to_virt;
use core::{
    borrow::{Borrow, BorrowMut},
    num::NonZeroUsize,
};
use log::info;
use spinlock::SpinNoIrq;
use xhci::{accessor::Mapper, extended_capabilities, ExtendedCapability, Registers};

use crate::host::structures::{dcbaa, scratchpad};

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

// pub(crate) fn new(xhci_behavior_impl: &'static mut dyn XhciBehaviors) {
//     let mut cont = Arc::new(Spinlock::new(xhci_behavior_impl));
//     let mut event_ring = Arc::new(Spinlock::new(EventRing::new(cont.clone())));
//     let mut command_ring = Arc::new(Spinlock::new(CommandRing::new(cont.clone())));

//     unsafe {
//         XHCI_CONTROLLER = Some(cont);
//         EVENT_RING = Some(event_ring);
//         COMMAND_RING = Some(command_ring);
//     }
//     // Ok(XhciController {
//     //     inner_impl: cont,
//     //     event_ring: event_ring,
//     //     command_ring: command_ring,
//     // })
// }

// pub(crate) fn init() {
//     if let Some(ref mut inited_controller) = unsafe { XHCI_CONTROLLER.borrow_mut() } {
//         let mut r = inited_controller.borrow_mut().lock().regs();
//         // get owner ship from bios(?)
//         if let Some(extended_caps) = inited_controller.lock().borrow_mut()..extra_features() {
//             let iter = extended_caps.into_iter();
//             for c in iter.filter_map(Result::ok) {
//                 if let ExtendedCapability::UsbLegacySupport(mut u) = c {
//                     let l = &mut u.usblegsup;
//                     l.update_volatile(|s| {
//                         s.set_hc_os_owned_semaphore();
//                     });

//                     while l.read_volatile().hc_bios_owned_semaphore()
//                         || !l.read_volatile().hc_os_owned_semaphore()
//                     {}
//                 }
//             }
//         }

//         info!("stop");
//         r.operational.usbcmd.update_volatile(|u| {
//             u.clear_run_stop();
//         });
//         info!("wait until halt");
//         while !r.operational.usbsts.read_volatile().hc_halted() {}
//         info!("start reset");
//         r.operational.usbcmd.update_volatile(|u| {
//             u.set_host_controller_reset();
//         });
//         info!("wait until reset complete");
//         while r.operational.usbcmd.read_volatile().host_controller_reset() {}
//         info!("wait until ready");
//         while r.operational.usbsts.read_volatile().controller_not_ready() {}

//         let n = r
//             .capability
//             .hcsparams1
//             .read_volatile()
//             .number_of_device_slots();
//         info!("setting num of slots:{}", n);
//         r.operational.config.update_volatile(|c| {
//             c.set_max_device_slots_enabled(n);
//         });

//         info!("init event ring...");
//         inited_controller
//             .into_inner()
//             .event_ring
//             .update_deq_with_xhci(r);
//         inited_controller.into_inner().event_ring.init_segtable(r);

//         info!("init command ring...");
//         inited_controller.into_inner().command_ring.lock().init();

//         dcbaa::init(r);
//         scratchpad::init_once(r);
//         command_exchanger::init(inited_controller.into_inner().command_ring.clone())
//     }
// }
