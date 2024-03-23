use core::f32::consts::E;

use conquer_once::spin::OnceCell;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::extended_capabilities::debug::EventRingDequeuePointer;

use crate::{dma, host::structures::event_ring};

use super::{
    event_ring::{EvtRing, TypeXhciTrb},
    registers, XHCI_CONFIG_IMODI,
};

struct ErstEntry {
    pub seg_base: usize,
    pub seg_size: u32,
    pub reserved: u32,
}

pub(crate) struct EventManager {
    event_ring: EvtRing,
    erst_entry: PageBox<ErstEntry>,
}

pub(crate) static EVENT_MANAGER: OnceCell<Spinlock<EventManager>> = OnceCell::uninit();

pub(crate) fn new() {
    registers::handle(|r| {
        let mut event_manager = EventManager {
            event_ring: EvtRing::new(),
            erst_entry: PageBox::new_slice(
                ErstEntry {
                    seg_base: 0,
                    seg_size: 0,
                    reserved: 0,
                },
                1,
            ),
        };
        let erst_ent = &mut event_manager.erst_entry;
        erst_ent.seg_base = event_manager.event_ring.get_ring_addr().as_usize();
        erst_ent.seg_size = event_manager.event_ring.get_trb_count();
        erst_ent.reserved = 0;

        let ir0 = r.interrupter_register_set.interrupter_mut(0);
        ir0.erstsz.update_volatile(|e| {
            e.set(1);
        });

        ir0.erstba.update_volatile(|b| {
            b.set(erst_ent.virt_addr().as_usize() as u64);
        });
        //TODO FIXIT
        ir0.erdp.update_volatile(|dp| {
            dp.set_event_ring_dequeue_pointer(
                event_manager.event_ring.get_ring_addr().as_usize() as u64
            );
        });
        ir0.imod.update_volatile(|im| {
            im.set_interrupt_moderation_interval(XHCI_CONFIG_IMODI);
        });
        ir0.iman.update_volatile(|im| {
            im.set_interrupt_enable();
        });

        EVENT_MANAGER
            .try_init_once(move || Spinlock::new(event_manager))
            .expect("Failed to initialize `EventManager`.");

        //     let slot_manager = SlotManager {
        //         dcbaa: PageBox::new_slice(VirtAddr::from(0 as usize), XHCI_CONFIG_MAX_SLOTS + 1),
        //         device: PageBox::new_slice(Device::new_64byte(), XHCI_CONFIG_MAX_SLOTS + 1),
        //     };

        //     r.operational
        //         .dcbaap
        //         .update_volatile(|d| d.set(slot_manager.dcbaa.virt_addr()));

        //     SLOT_MANAGER
        //         .try_init_once(move || Spinlock::new(slot_manager))
        //         .expect("Failed to initialize `SlotManager`.");
    });
}

pub(crate) fn handle_event() -> Result<TypeXhciTrb, ()> {
    if let Some(manager) = EVENT_MANAGER.get().unwrap().try_lock() {
        if let Some(trb) = manager.event_ring.get_deque_trb() {
            match trb {
                xhci::ring::trb::event::Allowed::TransferEvent(evt) => evt.completion_code(),
                xhci::ring::trb::event::Allowed::CommandCompletion(_) => todo!(),
                xhci::ring::trb::event::Allowed::PortStatusChange(_) => todo!(),
                xhci::ring::trb::event::Allowed::BandwidthRequest(_) => todo!(),
                xhci::ring::trb::event::Allowed::Doorbell(_) => todo!(),
                xhci::ring::trb::event::Allowed::HostController(_) => todo!(),
                xhci::ring::trb::event::Allowed::DeviceNotification(_) => todo!(),
                xhci::ring::trb::event::Allowed::MfindexWrap(_) => todo!(),
            }
        }
    }
    return Err(());
}
