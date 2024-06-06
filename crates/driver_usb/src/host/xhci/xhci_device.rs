use alloc::{sync::Arc, vec::Vec};
use log::debug;
use spinlock::SpinNoIrq;

use crate::{
    ax::{USBDeviceDriverOps, USBHostDriverOps},
    host::usb::descriptors::{self, Descriptor},
    OsDep,
};

use super::{event::Ring, Xhci};

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
    pub xhci: Arc<Xhci<O>>,
}

impl<O> DeviceAttached<O>
where
    O: OsDep,
{
    pub fn find_driver_impl<T: USBDeviceDriverOps<O>>(&mut self) -> Option<Arc<SpinNoIrq<T>>> {
        let device = self.fetch_desc_devices()[0]; //only pick first device desc
        T::try_create(self)
    }

    pub fn set_configuration(&mut self) {}

    pub fn fetch_desc_configs(&mut self) -> Vec<descriptors::desc_configuration::Configuration> {
        self.descriptors
            .iter()
            .filter_map(|desc| match desc {
                Descriptor::Configuration(config) => Some(config.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn fetch_desc_devices(&mut self) -> Vec<descriptors::desc_device::Device> {
        self.descriptors
            .iter()
            .filter_map(|desc| match desc {
                Descriptor::Device(device) => Some(device.clone()),
                _ => None,
            })
            .collect()
    }
}
