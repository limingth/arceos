use axhal::mem::VirtAddr;
use xhci::context::EndpointType;

use crate::host::{exchanger::transfer, page_box::PageBox, structures::descriptor};

pub(super) struct Default {
    sender: transfer::Sender,
}
impl Default {
    pub(super) fn new(sender: transfer::Sender) -> Self {
        Self { sender }
    }

    pub(super) fn ring_addr(&self) -> VirtAddr {
        self.sender.ring_addr()
    }

    pub(super) fn get_max_packet_size(&mut self) -> u16 {
        self.sender.get_max_packet_size_from_device_descriptor()
    }

    pub(super) fn get_raw_configuration_descriptors(&mut self) -> PageBox<[u8]> {
        self.sender.get_configuration_descriptor()
    }

    pub(super) fn set_configuration(&mut self, config_val: u8) {
        self.sender.set_configure(config_val);
    }

    pub(super) fn set_idle(&mut self) {
        self.sender.set_idle();
    }

    pub(super) fn set_boot_protocol(&mut self) {
        self.sender.set_boot_protocol();
    }

    pub(super) fn issue_nop_trb(&mut self) {
        self.sender.issue_nop_trb();
    }
}

pub(super) struct NonDefault {
    desc: descriptor::Endpoint,
    sender: transfer::Sender,
}
impl NonDefault {
    pub(super) fn new(desc: descriptor::Endpoint, sender: transfer::Sender) -> Self {
        Self { desc, sender }
    }

    pub(super) fn descriptor(&self) -> descriptor::Endpoint {
        self.desc
    }

    pub(super) fn transfer_ring_addr(&self) -> VirtAddr {
        self.sender.ring_addr()
    }

    pub(super) fn ty(&self) -> EndpointType {
        self.desc.ty()
    }

    pub(super) fn issue_normal_trb<T: ?Sized>(&mut self, b: &PageBox<T>) {
        self.sender.issue_normal_trb(b)
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    NoSuchEndpoint(EndpointType),
}
