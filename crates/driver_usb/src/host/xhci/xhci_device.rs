use alloc::{sync::Arc, vec::Vec};
use spinlock::SpinNoIrq;

use crate::{
    ax::{USBDeviceDriverOps, USBHostDriverOps},
    host::usb::descriptors,
    OsDep,
};

use super::event::Ring;

pub struct DeviceAttached<O>
where
    O: OsDep,
{
    pub hub: usize,
    pub port: usize,
    pub num_endp: usize,
    pub address: usize,
    pub transfer_rings: Vec<Ring<O>>,
    pub descriptors: Vec<descriptors::Descriptor>,
}

impl<O> DeviceAttached<O>
where
    O: OsDep,
{
    pub fn find_driver_impl<T: USBDeviceDriverOps<O>>(&mut self) -> Option<Arc<SpinNoIrq<T>>> {
        T::try_create(self)
    }
}
