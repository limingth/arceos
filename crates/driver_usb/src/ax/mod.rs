use core::alloc::Allocator;

use crate::host::xhci::xhci_device::DeviceAttached;
use crate::host::Controller;
use crate::host::{xhci::Xhci, USBHost};
use crate::OsDep;
use alloc::sync::Arc;
use axalloc::GlobalNoCacheAllocator;
use driver_common::*;
use spinlock::SpinNoIrq;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct USBHostInfo {}

pub trait USBDeviceDriverOps<O: OsDep> {
    fn try_create(device: &mut DeviceAttached<O>) -> Option<Arc<SpinNoIrq<Self>>>;

    fn work(&self, xhci: &Xhci<O>); //should return a process handle, but since our async feature is broken, lets just block the main thread
}

/// Operations that require a graphics device driver to implement.
pub trait USBHostDriverOps: BaseDriverOps {}

impl<O> BaseDriverOps for USBHost<O>
where
    O: OsDep<DMA = GlobalNoCacheAllocator>,
{
    fn device_name(&self) -> &str {
        "USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}
