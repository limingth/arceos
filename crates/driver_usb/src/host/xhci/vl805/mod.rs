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
    types::{Bar, ConfigKind, ConfigCommand,ConfigSpace},
    PciAddress,
};
use log::{debug, info};

const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;

pub struct VL805 {
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
    fn new(mmio_base: usize) -> Self {
        let mapper = MemoryMapper;
        let regs = unsafe { xhci::Registers::new(mmio_base, mapper) };
        let version = regs.capability.hciversion.read_volatile();
        debug!("xhci version: {:x}", version.get());
        let mut o = regs.operational;
        debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("xhci wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        info!("xhci ok");

        o.usbcmd.update_volatile(|f|{
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        info!("XHCI reset HC");

        VL805 { }
    }
}

impl VL805 {
    pub fn probe_pci<A: Allocator+Clone>(config: &ConfigSpace, dma_alloc: A) -> Option<Self> {
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
                let mut dma: DMAVec<A, u8> = DMAVec::new(0x100, 0x1000, dma_alloc.clone());
                let mbox = Mailbox::new();
                let msg = MsgNotifyXhciReset {};
                mbox.send(&msg, &mut dma);

                debug!("VL805 @0x{:X}", address);
                config.header.set_command([
                    ConfigCommand::MemorySpaceEnable,
                    ConfigCommand::BusMasterEnable,
                    ConfigCommand::ParityErrorResponse,
                    ConfigCommand::SERREnable,
                ]);
                let vl805 = VL805::new(address as _);
                return Some(vl805);
            }
        }

        None
    }
}
