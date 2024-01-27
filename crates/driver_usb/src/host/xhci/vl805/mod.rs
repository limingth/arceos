use core::alloc::Layout;

use super::MemoryMapper;
pub use crate::host::USBHostDriverOps;
use axalloc::{global_add_free_memory, global_allocator};
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
    pub fn probe_pci(
        config: &ConfigSpace,
        dma_alloc: &impl alloc::alloc::Allocator
    ) -> Option<Self> {
        let (vendor_id, device_id) = config.header.vendor_id_and_device_id();
        if !(vendor_id == VL805_VENDOR_ID && device_id == VL805_DEVICE_ID) {
            return None;
        }
        let vl805 = VL805::new(config.address);
        let allocator = global_allocator();
        let dma_addr = dma_alloc.allocate(Layout::from_size_align(0x100, 0x1000).unwrap()).unwrap();
        debug!("dma: {:p} ", dma_addr);



        Some(vl805)
    }
}