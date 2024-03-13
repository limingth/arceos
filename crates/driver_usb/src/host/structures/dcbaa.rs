// SPDX-License-Identifier: GPL-3.0-or-later

use {
    super::registers,
    axhal::mem::VirtAddr,
    conquer_once::spin::Lazy,
    core::ops::{Index, IndexMut},
    page_box::PageBox,
    spinning_top::Spinlock,
};

static DCBAA: Lazy<Spinlock<DeviceContextBaseAddressArray>> =
    Lazy::new(|| Spinlock::new(DeviceContextBaseAddressArray::new()));

pub(crate) fn init() {
    DCBAA.lock().init();
}

pub(crate) fn register(port_id: usize, a: VirtAddr) {
    DCBAA.lock()[port_id] = a;
}

pub(crate) struct DeviceContextBaseAddressArray {
    arr: PageBox<[VirtAddr]>,
}
impl DeviceContextBaseAddressArray {
    fn new() -> Self {
        // let arr = PageBox::new_slice(VirtAddr::from(0 as usize), Self::num_of_slots());
        let arr = PageBox::alloc_pages(1, VirtAddr::from(0 as usize));
        //just alloc 1 page with full of zeros directly

        Self { arr }
    }

    fn init(&self) {
        self.register_address_to_xhci_register();
    }

    fn num_of_slots() -> usize {
        registers::handle(|r| {
            r.capability
                .hcsparams1
                .read_volatile()
                .number_of_device_slots()
                + 1
        })
        .into()
    }

    fn register_address_to_xhci_register(&self) {
        registers::handle(|r| {
            let _ = &self;
            r.operational.dcbaap.update_volatile(|d| {
                let _ = &self;
                d.set(self.virt_addr().as_usize() as u64);
            });
        });
    }

    fn virt_addr(&self) -> VirtAddr {
        self.arr.virt_addr()
    }
}
impl Index<usize> for DeviceContextBaseAddressArray {
    type Output = VirtAddr;
    fn index(&self, index: usize) -> &Self::Output {
        &self.arr[index]
    }
}
impl IndexMut<usize> for DeviceContextBaseAddressArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.arr[index]
    }
}
