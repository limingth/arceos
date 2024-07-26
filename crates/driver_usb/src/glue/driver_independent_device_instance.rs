use core::hash::Hash;

use crate::{
    abstractions::PlatformAbstractions,
    host::data_structures::{host_controllers::ControllerArc, MightBeInited},
    usb::descriptors::TopologicalUSBDescriptorRoot,
};

#[derive(Clone)]
pub struct DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub slotid: usize,
    pub descriptors: MightBeInited<TopologicalUSBDescriptorRoot>,
    pub controller: ControllerArc<O>,
}

impl<O> DriverIndependentDeviceInstance<O>
where
    O: PlatformAbstractions,
{
    pub fn new(slotid: usize, controller: ControllerArc<O>) -> Self {
        Self {
            slotid: slotid,
            descriptors: MightBeInited::default(),
            controller: controller,
        }
    }
}
