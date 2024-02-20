use core::ops::{Index, IndexMut};

use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
use axhal::mem::PhysAddr;
use spinlock::SpinNoIrq;
use xhci::{accessor::Mapper, registers::doorbell::Register, Registers};

pub static DCBAA: Option<Arc<SpinNoIrq<DeviceContextBaseAddressArray>>> = None;

pub(crate) fn init(r: &Registers<impl Mapper + Clone>) {
    let slot_count = r
        .capability
        .hcsparams1
        .read_volatile()
        .number_of_device_slots()
        + 1;
    unsafe {
        DCBAA = Some(Arc::new(SpinNoIrq::new(
            DeviceContextBaseAddressArray::new(slot_count as usize),
        )))
    }
}

pub(crate) struct DeviceContextBaseAddressArray {
    devices: Vec<PhysAddr>,
}

impl DeviceContextBaseAddressArray {
    fn new(slot_count: usize) -> Self {
        let arr = vec![(0 as usize).into(); slot_count];
        Self { devices: arr }
    }

    fn init(&self, register: &mut Registers<impl Mapper + Clone>) {
        self.register_address_to_xhci_register(register);
    }

    fn register_address_to_xhci_register(&self, r: &mut Registers<impl Mapper + Clone>) {
        let _ = &self;
        r.operational.dcbaap.update_volatile(|d| {
            let _ = &self;
            d.set(self.phys_addr().as_usize() as u64);
        });
    }

    fn phys_addr(&self) -> PhysAddr {
        self.devices.as_ptr().addr().into()
    }
}

impl Index<usize> for DeviceContextBaseAddressArray {
    type Output = PhysAddr;
    fn index(&self, index: usize) -> &Self::Output {
        &self.devices[index]
    }
}
impl IndexMut<usize> for DeviceContextBaseAddressArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.devices[index]
    }
}
