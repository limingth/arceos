use super::MemoryMapper;
pub use crate::host::USBHostDriverOps;
use axhal::{
    cpu,
    mem::{phys_to_virt, PhysAddr, VirtAddr},
};
use driver_pci::PciAddress;
use driver_common::*;


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
    pub fn probe_pci(
        vendor_id: u16,
        device_id: u16,
        bdf: PciAddress,
        bar: usize,
    ) -> Option<Self> {
        if !(vendor_id == VL805_VENDOR_ID && device_id == VL805_DEVICE_ID) {
            return None;
        }
        let vl805 = VL805::new(bdf);


        Some(vl805)
    }
}