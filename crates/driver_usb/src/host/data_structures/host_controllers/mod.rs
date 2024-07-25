pub mod xhci;
use alloc::{boxed::Box, sync::Arc};
use spinlock::SpinNoIrq;

use crate::{
    abstractions::{OSAbstractions, PlatformAbstractions},
    err::Result,
    USBSystemConfig,
};

pub trait Controller<O>: Send
where
    O: PlatformAbstractions,
{
    fn new(config: USBSystemConfig<O>) -> Self
    where
        Self: Sized;

    fn init(&mut self);
    fn probe(&mut self);

    // fn poll();
}

pub(crate) type ControllerArc<O> = Arc<SpinNoIrq<Box<dyn Controller<O>>>>;
