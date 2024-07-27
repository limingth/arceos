use alloc::vec::Vec;

use crate::usb::urb::URB;

pub trait USBSystemDriverModule<'a>: Send + Sync {
    fn gather_urb(self: &Self) -> Option<URB<'a>> {
        None
    }

    fn preload_module(&self);
}
