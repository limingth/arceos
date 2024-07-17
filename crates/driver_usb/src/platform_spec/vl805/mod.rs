use core::{
    alloc::{Allocator, Layout},
    cell::LazyCell,
    ptr::slice_from_raw_parts_mut,
};
mod mailbox;
use crate::{
    host::{xhci::Xhci, USBHost, USBHostConfig},
    OsDep,
};

use self::mailbox::*;
use driver_common::*;
use driver_pci::{
    types::{Bar, ConfigCommand, ConfigKind, ConfigSpace},
    PciAddress,
};
use log::{debug, info};

const VL805_VENDOR_ID: u16 = 0x1106;
const VL805_DEVICE_ID: u16 = 0x3483;

pub struct VL805<O>
where
    O: OsDep,
{
    host: USBHost<O>,
}

impl<O> BaseDriverOps for VL805<O>
where
    O: OsDep,
{
    fn device_name(&self) -> &str {
        "VL805 4-Port USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}

impl<O> VL805<O>
where
    O: OsDep + 'static,
{
    // fn new(mmio_base: usize, alloc: A) -> Self {}
    pub fn probe_pci(config: &ConfigSpace, osdep_operations: O) -> Option<Self> {
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
                {
                    // let mut dma: DMAVec<A, u8> = DMAVec::new(0x100, 0x1000, osdep_operations.clone());
                    let mut dma = osdep_operations
                        .dma_alloc()
                        .allocate_zeroed(Layout::from_size_align(0x100, 0x1000).unwrap())
                        .unwrap();
                    let mbox = Mailbox::new();
                    let msg = MsgNotifyXhciReset {};
                    mbox.send(&msg, unsafe { dma.as_mut() });

                    debug!("VL805 @0x{:X}", address);
                    config.header.set_command([
                        ConfigCommand::MemorySpaceEnable,
                        ConfigCommand::BusMasterEnable,
                        ConfigCommand::ParityErrorResponse,
                        ConfigCommand::SERREnable,
                    ]);
                }
                return Some(VL805::<O> {
                    host: USBHost::new::<Xhci<_>>({
                        USBHostConfig::<O>::new(address as _, 30, 0, osdep_operations)
                    })
                    .unwrap(),
                });
            }
        }

        None
    }
}
