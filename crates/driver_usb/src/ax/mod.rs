use driver_common::*;
use crate::host::Controller;
use crate::host::{xhci::Xhci, USBHost};

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct USBHostInfo {}

/// Operations that require a graphics device driver to implement.
pub trait USBHostDriverOps: BaseDriverOps {}


impl BaseDriverOps for USBHost {
    fn device_name(&self) -> &str {
        "USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}