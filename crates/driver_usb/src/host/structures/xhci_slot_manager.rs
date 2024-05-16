use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::context::{Device, Device64Byte, DeviceHandler};

use super::registers;

const XHCI_CONFIG_MAX_SLOTS: usize = 64;
pub(crate) struct SlotManager {
    dcbaa: PageBox<[VirtAddr]>,
    device: PageBox<[Device64Byte]>,
}

impl SlotManager {
    pub fn assign_device(&mut self, valid_slot_id: u8, device: Device64Byte) {
        self.device[valid_slot_id as usize - 1] = device;
        self.dcbaa[valid_slot_id as usize] = unsafe {
            self.device
                .as_ptr()
                .offset((valid_slot_id as usize - 1) as isize)
        }
        //TODO 需要考虑内存同步问题
    }
}

pub(crate) static SLOT_MANAGER: OnceCell<Spinlock<SlotManager>> = OnceCell::uninit();

pub(crate) fn transfer_event(
    uch_completion_code: u8,
    n_transfer_length: u32,
    uch_slot_id: u8,
    uch_endpoint_id: u8,
) {
    assert!((1 <= uch_slot_id) && (usize::from(uch_slot_id) <= XHCI_CONFIG_MAX_SLOTS));
    // TODO: check device exists
    let slot_manager = SLOT_MANAGER.try_get().unwrap().lock();
    let device = &slot_manager.device[(uch_slot_id - 1) as usize] as &Device64Byte;
    let endpoint = device.endpoint((uch_endpoint_id - 1).try_into().unwrap());
    // TODO: event transfer
}

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
    debug!("initialized!");
}

pub fn set_dcbaa(buffer_array: &[VirtAddr]) {
    let mut dcbaa_box = &mut SLOT_MANAGER.get().unwrap().lock().dcbaa;
    buffer_array
        .iter()
        .zip(dcbaa_box.iter_mut())
        .for_each(|(l, r)| *r = *l);
}
