use core::sync::atomic::AtomicUsize;

use crate::PlatformAbstractions;

use super::trasnfer::control::ControlTransfer;

//basiclly migrated version of linux urb
struct URB {
    uid: usize,
    device: usize,
    operation: RequestedOperation,
}

pub enum RequestedOperation {
    Control(ControlTransfer),
    Bulk,
    Interrupt,
    Isoch,
}
