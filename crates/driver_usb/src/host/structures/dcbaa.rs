use crate::host::page_box::PageBox;

use super::registers;
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use core::ops::DerefMut;
use spinning_top::Spinlock;

static DCBAA: OnceCell<Spinlock<PageBox<[VirtAddr]>>> = OnceCell::uninit();

pub fn init() {
    DCBAA.init_once(|| Spinlock::new(PageBox::new_slice(VirtAddr::from(0), array_len())));

    registers::handle(|r| {
        r.operational.dcbaap.update_volatile(|d| {
            d.set(lock().virt_addr().as_usize() as u64);
        })
    })
}

pub fn register_device_context_addr(port_id: usize, a: VirtAddr) {
    assert_ne!(port_id, 0, "A port ID must be greater than 0.");

    lock()[port_id] = a;
}

pub fn register_scratchpad_addr(a: VirtAddr) {
    lock()[0] = a;
}

fn lock() -> impl DerefMut<Target = PageBox<[VirtAddr]>> {
    DCBAA
        .try_get()
        .expect("`DCBAA` is not initialized.")
        .try_lock()
        .expect("Failed to lock `DCBAA`.")
}

fn array_len() -> usize {
    registers::handle(|r| {
        r.capability
            .hcsparams1
            .read_volatile()
            .number_of_device_slots()
            + 1
    })
    .into()
}
