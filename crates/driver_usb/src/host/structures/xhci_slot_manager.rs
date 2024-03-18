use alloc::{sync::Arc, vec};
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::context::{Device, Device64Byte};

use super::registers;

const XHCI_CONFIG_MAX_SLOTS: usize = 64;
struct SlotManager {
    dcbaa: PageBox<[VirtAddr]>,
    device: PageBox<[xhci::context::Device64Byte]>,
}

static SLOT_MANAGER: OnceCell<Spinlock<SlotManager>> = OnceCell::uninit();

pub(crate) fn new() {
    registers::handle(|r| {
        let slot_manager = SlotManager {
            dcbaa: PageBox::new_slice(VirtAddr::from(0 as usize), XHCI_CONFIG_MAX_SLOTS + 1),
            device: PageBox::new_slice(Device::new_64byte(), XHCI_CONFIG_MAX_SLOTS + 1),
        };

        r.operational
            .dcbaap
            .update_volatile(|d| d.set(slot_manager.dcbaa.virt_addr().as_usize() as u64));

        SLOT_MANAGER
            .try_init_once(move || Spinlock::new(slot_manager))
            .expect("Failed to initialize `SlotManager`.");
    });
}
