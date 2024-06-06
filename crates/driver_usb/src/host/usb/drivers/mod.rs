pub mod driver_usb_hid;
// use core::cell::OnceCell;

// use alloc::{boxed::Box, sync::Arc, vec::Vec};
// use spinlock::SpinNoIrq;

// use crate::{ax::USBDeviceDriverOps, OsDep};

// use super::xhci::xhci_device::DeviceAttached;

// // struct DriverBus<O: OsDep> {
// //     bus: Vec<Box<dyn Fn(&mut DeviceAttached<O>) -> Option<Arc<dyn USBDeviceDriverOps<O>>>>>,
// // } //TODO: DESIGN Driver Bus
