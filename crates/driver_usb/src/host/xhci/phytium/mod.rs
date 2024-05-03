use core::alloc::Allocator;
mod mailbox;
use crate::{
    device_types::PCI_DEVICE_ID_PHYTIUM_XHCI,
    dma::DMAVec,
    // host::xhci::phytium::mailbox::{Mailbox, MsgNotifyXhciReset},
};
use driver_common::*;
use driver_pci::{
    device_types::PCI_VENDOR_ID_PHYTIUM,
    types::{Bar, ConfigCommand, ConfigKind, ConfigSpace},
};
use log::debug;

pub struct PhytiymXHCIController<A: Allocator + Clone> {
    alloc: A,
    // regs: Registers<MemoryMapper>,
    // extended_capabilities: Option<extended_capabilities::List<MemoryMapper>>,
    base_addr: usize,
}

impl<A: Allocator + Clone + Sync + Send> BaseDriverOps for PhytiymXHCIController<A> {
    fn device_name(&self) -> &str {
        "Phytium pi 4-Port USB Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}

impl<A: Allocator + Clone> PhytiymXHCIController<A> {
    fn new(mmio_base: usize, alloc: A) -> Self {
        super::xhci_operations::init(mmio_base);
        PhytiymXHCIController {
            base_addr: mmio_base,
            alloc,
        }
    }

    pub fn probe_pci(config: &ConfigSpace, dma_alloc: A) -> Option<Self> {
        let (vendor_id, device_id) = config.header.vendor_id_and_device_id();
        let revision_and_class = config.header.revision_and_class();
        if !((vendor_id as usize) == PCI_VENDOR_ID_PHYTIUM
            && device_id == PCI_DEVICE_ID_PHYTIUM_XHCI)
        {
            return None;
        }

        //THINK: may be we could use pattern match instead of optional compile?
        // match Some((vendor_id as usize, device_id)) {
        //     Some((VL805_VENDOR_ID, VL805_DEVICE_ID)) => {
        if let ConfigKind::Endpoint { inner } = &config.kind {
            let bar = inner.bar(0).unwrap();
            if let Bar::Memory64 {
                address,
                size,
                prefetchable,
            } = bar
            {
                let mut dma: DMAVec<A, u8> = DMAVec::new(0x100, 0x1000, dma_alloc.clone());
                // let mbox = Mailbox::new();
                // let msg = MsgNotifyXhciReset {};
                // mbox.send(&msg, &mut dma);

                debug!("Phytium xhci @0x{:X}", address);
                config.header.set_command([
                    ConfigCommand::MemorySpaceEnable,
                    ConfigCommand::BusMasterEnable,
                    ConfigCommand::ParityErrorResponse,
                    ConfigCommand::SERREnable,
                ]);
                let phytium = PhytiymXHCIController::new(address as _, dma_alloc);
                return Some(phytium);
            }
        }
        // }
        // Some(_) => (),
        // None => (),
        // }
        None
    }
}
