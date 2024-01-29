use core::{
    alloc::{Allocator, Layout},
    ptr::slice_from_raw_parts_mut,
};
mod mailbox;
use self::mailbox::*;
use super::MemoryMapper;
use crate::dma::DMAVec;
pub use crate::host::USBHostDriverOps;
use driver_common::*;
use driver_pci::{
    types::{Bar, ConfigKind, ConfigSpace},
    PciAddress,
};
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
    pub fn probe_pci<A: Allocator>(config: &ConfigSpace, dma_alloc: &A) -> Option<Self> {
        let (vendor_id, device_id) = config.header.vendor_id_and_device_id();
        if !(vendor_id == VL805_VENDOR_ID && device_id == VL805_DEVICE_ID) {
            return None;
        }

        if let ConfigKind::Endpoint { inner } = &config.kind {
            let bar = inner.bar(0).unwrap();
            if let Bar::Memory64 {
                address,
                size,
                prefetchable,
            } = bar
            {
                let mut dma: DMAVec<'_, A, u8> = DMAVec::new(0x100, 0x1000, dma_alloc);
                let mbox = Mailbox::new();
                let msg = MsgNotifyXhciReset {};
                mbox.send(&msg, &mut dma);

                debug!("VL805 @0x{:X}", address);

                let vl805 = VL805::new(config.address);
                return Some(vl805);
            }
        }

        None
    }
}
