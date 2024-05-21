use core::alloc::Allocator;

use crate::host::Controller;
use crate::host::{xhci::Xhci, USBHost};
use crate::OsDep;
use driver_common::*;
use axalloc::GlobalNoCacheAllocator;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct USBHostInfo {}

/// Operations that require a graphics device driver to implement.
pub trait USBHostDriverOps: BaseDriverOps {}

impl <O>BaseDriverOps for USBHost<O>
where O: OsDep<DMA=GlobalNoCacheAllocator>
{
    fn device_name(&self) -> &str {
        "USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}
