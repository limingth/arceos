use alloc::vec::Vec;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance, usb::urb::URB,
};

pub trait USBSystemDriverModule<'a, O>: Send + Sync
where
    O: PlatformAbstractions,
{
    fn gather_urb(self: &Self) -> Option<URB<'a, O>> {
        None
    }

    fn should_active(independent_dev: DriverIndependentDeviceInstance<O>) -> Option<Vec<Self>>
    where
        Self: Sized;

    fn preload_module(&self);
}
