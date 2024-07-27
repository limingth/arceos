use alloc::vec::Vec;

use crate::usb::urb::URB;

pub struct USBSubSystemDriverModule {
    gather_urb: dyn FnMut(&mut Self) -> UsbRequestGroup,
    preload_module: dyn FnMut(&mut Self),
    postload_module: dyn FnMut(&mut Self),
}

pub type UsbRequestGroup = Vec<URB>;

