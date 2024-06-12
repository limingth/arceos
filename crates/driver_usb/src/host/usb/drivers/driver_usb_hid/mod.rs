use core::marker::PhantomData;

use alloc::sync::Arc;
use driver_common::BaseDriverOps;
use log::debug;
use num_traits::ToPrimitive;
use spinlock::SpinNoIrq;
use xhci::ring::trb::transfer::{Direction, TransferType};

use crate::{
    ax::USBDeviceDriverOps,
    dma::DMA,
    host::{
        usb::descriptors::{
            self, desc_device::USBDeviceClassCode, DescriptionTypeIndexPairForControlTransfer,
            Descriptor,
        },
        xhci::{xhci_device::DeviceAttached, Xhci},
    },
    OsDep,
};

const TAG: &str = "[USB-HID DRIVER]";

pub struct USBDeviceDriverHidMouseExample<O>
where
    O: OsDep,
{
    hub: usize,
    port: usize,
    slot: usize,
    xhci: Arc<Xhci<O>>,
}

impl<O> USBDeviceDriverHidMouseExample<O>
where
    O: OsDep,
{
    fn operate_device<F, T>(&self, mut op: F) -> T
    where
        F: Fn(&mut DeviceAttached<O>) -> T,
    {
        op(self
            .xhci
            .dev_ctx
            .lock()
            .attached_set
            .get_mut(&(self.slot - 1))
            .unwrap())
    }
}

impl<O> USBDeviceDriverOps<O> for USBDeviceDriverHidMouseExample<O>
where
    O: OsDep,
{
    fn try_create(
        device: &mut DeviceAttached<O>,
    ) -> Option<alloc::sync::Arc<spinlock::SpinNoIrq<Self>>> {
        debug!("creating!");
        // device
        //     .fetch_desc_devices()
        //     .first_mut()
        //     .map(|device_desc| {
        //         if device_desc.class == USBDeviceClassCode::HID.to_u8().unwrap() {
        //             Some(Arc::new(SpinNoIrq::new(Self {
        //                 hub: device.hub,
        //                 port: device.port,
        //                 slot: device.slot_id,
        //                 xhci: device.xhci.clone(),
        //             })))
        //         } else {
        //             None
        //         }
        //     })
        //     .unwrap()
        if device.has_desc(|desc| {
            if let Descriptor::Hid(hid) = desc {
                true
            } else {
                false
            }
        }) {
            let arc = Some(Arc::new(SpinNoIrq::new(Self {
                hub: device.hub,
                port: device.port,
                slot: device.slot_id,
                xhci: device.xhci.clone(),
            })));
            debug!("create!");
            return arc;
        }
        debug!("nothing!");
        None
    }

    fn work(&self) {
        let interface_in_use =
            self.operate_device(|dev| dev.fetch_desc_interfaces()[dev.current_interface].clone());
        let buffer = DMA::new_singleton_page4k(0u8, self.xhci.config.os.dma_alloc());
        let idle_req = self.xhci.construct_control_transfer_req(
            &buffer,
            0x0, //CLASS
            0xA, //SET IDLE
            descriptors::DescriptorType::Hid.value_for_transfer_control_index((1 << 8 | 0) as u8), //duration 1, report 0: 1<<8 | 0
            interface_in_use.interface_number as u16,
            (TransferType::No, Direction::Out),
        );

        {
            debug!("{TAG}: post idle request");
            let result = self.operate_device(|dev| {
                self.xhci.post_control_transfer(
                    idle_req,
                    dev.transfer_rings.get_mut(0).unwrap(),
                    1,
                    dev.slot_id,
                )
            });
            debug!("{TAG}: result: {:?}", result);
            // debug!("{TAG} buffer: {:?}", buffer);
        }

        debug!("waiting for event!");
        let busy_wait_for_event = self.xhci.busy_wait_for_event();
        debug!("getted: {:?}", busy_wait_for_event);
    }
}
