use core::hash::Hash;

use crate::{
    abstractions::PlatformAbstractions, host::data_structures::host_controllers::ControllerArc,
};

#[derive(Clone)]
pub struct DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub slotid: usize,
    controller: ControllerArc<O>,
}

impl<O> DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub fn new(slotid: usize, controller: ControllerArc<O>) -> Self {
        Self {
            slotid: slotid,
            controller: controller,
        }
    }
}
