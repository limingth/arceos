use core::sync::atomic::AtomicUsize;

use crate::PlatformAbstractions;

use super::trasnfer::control::ControlTransfer;

//basiclly migrated version of linux urb
struct URB<O>
where
    O: PlatformAbstractions,
{
    uid: usize,
    device: usize,
    operation: RequestedOperation<O>,
}

pub enum RequestedOperation<O>
where
    O: PlatformAbstractions,
{
    Control(ControlTransfer<O>),
    Bulk,
    Interrupt,
    Isoch,
}
