use core::marker::PhantomData;

use driver_common::BaseDriverOps;
use num_traits::ToPrimitive;

use crate::{
    ax::USBDeviceDriverOps, host::usb::descriptors::desc_device::USBDeviceClassCode, OsDep,
};

pub struct USBDeviceDriverHidKeyboardExample<O>
where
    O: OsDep,
{
    _phantomdata: PhantomData<O>,
}

impl<O> USBDeviceDriverOps<O> for USBDeviceDriverHidKeyboardExample<O>
where
    O: OsDep,
{
    fn try_create(
        device: &mut crate::host::xhci::xhci_device::DeviceAttached<O>,
    ) -> Option<alloc::sync::Arc<spinlock::SpinNoIrq<Self>>> {
        device
            .fetch_desc_devices()
            .first_mut()
            .map(|device| {
                if device.class == USBDeviceClassCode::HID.to_u8().unwrap() {
                    None //TODO do something
                } else {
                    None
                }
            })
            .unwrap()
    }
}
