pub mod xhci;
use core::sync::atomic::{fence, Ordering};

use ::xhci::{
    context::{EndpointType, Input},
    ring::trb::command,
};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use log::trace;
use spinlock::SpinNoIrq;

use crate::{
    abstractions::{dma::DMA, OSAbstractions, PlatformAbstractions},
    err::Result,
    usb::trasnfer::control::ControlTransfer,
    USBSystemConfig,
};

pub trait Controller<O>: Send
where
    O: PlatformAbstractions,
{
    fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self
    where
        Self: Sized;

    fn init(&mut self);
    fn probe(&mut self) -> Vec<usize>;
    fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result;

    fn device_slot_assignment(&mut self) -> usize;
    fn address_device(&mut self, slot_id: usize, port_id: usize);
    fn control_fetch_control_point_packet_size(&mut self, slot_id: usize) -> u8;
    fn set_ep0_packet_size(&mut self, dev_slot_id: usize, max_packet_size: u16);
}

pub(crate) type ControllerArc<O> = Arc<SpinNoIrq<Box<dyn Controller<O>>>>;
