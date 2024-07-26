use core::sync::atomic::AtomicUsize;

use crate::PlatformAbstractions;

use super::{operation::Configuration, trasnfer::control::ControlTransfer};

//basiclly migrated version of linux urb
pub struct URB<'a> {
    pub device_slot_id: usize,
    pub operation: RequestedOperation<'a>,
}

impl<'a> URB<'a> {
    pub fn new(device_slot_id: usize, op: RequestedOperation<'a>) -> Self {
        Self {
            device_slot_id,
            operation: op.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RequestedOperation<'a> {
    Control(ControlTransfer),
    Bulk,
    Interrupt,
    Isoch,
    ConfigureDevice(Configuration<'a>),
}
