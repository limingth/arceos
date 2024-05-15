use alloc::sync::Arc;
use spinning_top::Spinlock;

use super::{xhci_roothub::RootPort, USBSpeed};

pub struct XHCIUSBDevice {}

impl XHCIUSBDevice {
    pub fn new(port_speed: USBSpeed, root_port: Arc<Spinlock<RootPort>>) -> Self {
        todo!()
    }
}
