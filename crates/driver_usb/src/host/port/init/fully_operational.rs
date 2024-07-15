use crate::host::{
    page_box::PageBox,
    port::endpoint::{self, Error, NonDefault},
    structures::descriptor::Descriptor,
};

use super::endpoints_initializer::EndpointsInitializer;
use alloc::vec::Vec;
use core::slice;
use log::debug;
use xhci::context::EndpointType;

pub struct FullyOperational {
    descriptors: Vec<Descriptor>,
    def_ep: endpoint::Default,
    eps: Vec<NonDefault>,
}
impl FullyOperational {
    pub(super) fn new(i: EndpointsInitializer) -> Self {
        let descriptors = i.descriptors();
        let (def_ep, eps) = i.endpoints();

        debug!("Endpoints collected");

        Self {
            descriptors,
            def_ep,
            eps,
        }
    }

    pub(in super::super) fn ty(&self) -> (u8, u8, u8) {
        for d in &self.descriptors {
            if let Descriptor::Interface(i) = d {
                return i.ty();
            }
        }

        unreachable!("HID class must have at least one interface descriptor");
    }

    pub(in super::super) fn issue_normal_trb(
        &mut self,
        b: &PageBox<impl ?Sized>,
        ty: EndpointType,
    ) -> Result<(), Error> {
        for ep in &mut self.eps {
            if ep.ty() == ty {
                ep.issue_normal_trb(b);
                return Ok(());
            }
        }

        Err(Error::NoSuchEndpoint(ty))
    }

    pub(in super::super) fn issue_nop_trb(&mut self) {
        self.def_ep.issue_nop_trb();
    }

    pub(in super::super) fn set_configure(&mut self, config_val: u8) {
        self.def_ep.set_configuration(config_val);
    }

    pub(in super::super) fn set_idle(&mut self) {
        self.def_ep.set_idle();
    }

    pub(in super::super) fn set_boot_protocol(&mut self) {
        self.def_ep.set_boot_protocol();
    }

    pub(in super::super) fn descriptors(&self) -> &[Descriptor] {
        &self.descriptors
    }
}
impl<'a> IntoIterator for &'a mut FullyOperational {
    type Item = &'a mut NonDefault;
    type IntoIter = slice::IterMut<'a, NonDefault>;

    fn into_iter(self) -> Self::IntoIter {
        self.eps.iter_mut()
    }
}
