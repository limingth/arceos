use core::{alloc::{Allocator, Layout}, ptr::slice_from_raw_parts_mut};
mod mailbox;
use self::mailbox::*;

use super::MemoryMapper;
use crate::dma::DMAVec;
pub use crate::host::USBHostDriverOps;
use axhal::{
    cpu,
    mem::{phys_to_virt, PhysAddr, VirtAddr},
};
use driver_pci::{types::ConfigSpace, PciAddress};
use driver_common::*;
use log::debug;


const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;

pub struct VL805 {
    bdf: PciAddress,
}

impl BaseDriverOps for VL805 {
    fn device_name(&self) -> &str {
        "VL805 4-Port USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}

impl VL805 {
    fn new(bdf: PciAddress) -> Self {
        VL805 { bdf }
    }
}

impl VL805 {
    pub fn probe_pci<A: Allocator>(
        config: &ConfigSpace,
        dma_alloc: &A
    ) -> Option<Self> {
        let (vendor_id, device_id) = config.header.vendor_id_and_device_id();
        if !(vendor_id == VL805_VENDOR_ID && device_id == VL805_DEVICE_ID) {
            return None;
        }
        let mut dma: DMAVec<'_, A, u8> = DMAVec::new(0x100, 0x1000, dma_alloc);
        let mbox = Mailbox::new();
        let msg = MsgNotifyXhciReset{};
        // let msg = MsgGetFirmwareRevision{};
        mbox.send(&msg,  &mut dma);
        let vl805 = VL805::new(config.address);
        Some(vl805)
    }
}