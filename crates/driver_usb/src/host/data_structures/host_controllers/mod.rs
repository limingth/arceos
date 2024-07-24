use alloc::{boxed::Box, sync::Arc};
use spinlock::SpinNoIrq;

use crate::{
    abstractions::{OSAbstractions, PlatformAbstractions},
    err::Result,
    host::USBHostConfig,
};

pub mod xhci;
pub trait Controller<O>: Send
where
    O: PlatformAbstractions,
{
    fn new(config: USBHostConfig<O>) -> Result<Self>
    where
        Self: Sized;
}

pub(crate) type ControllerArc<O> = Arc<SpinNoIrq<Box<dyn Controller<O>>>>;
