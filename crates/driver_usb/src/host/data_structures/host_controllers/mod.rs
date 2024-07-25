pub mod xhci;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
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
    fn device_slot_assignment(&mut self) -> usize;
    fn address_device(&mut self, slot_id: usize, port_id: usize);
    fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result;

    fn control_fetch_control_point_packet_size(&mut self, slot_id: usize) -> u16;
    // fn poll();
}

pub(crate) type ControllerArc<O> = Arc<SpinNoIrq<Box<dyn Controller<O>>>>;
