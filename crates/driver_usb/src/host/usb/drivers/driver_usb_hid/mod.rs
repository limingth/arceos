use core::marker::PhantomData;

use alloc::sync::Arc;
use driver_common::BaseDriverOps;
use log::debug;
use num_traits::{FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::ring::trb::transfer::{Direction, TransferType};

use crate::{
    ax::USBDeviceDriverOps,
    dma::DMA,
    host::{
        usb::descriptors::{
            self, desc_device::USBDeviceClassCode, desc_hid::USBHIDSubClassDescriptorType,
            DescriptionTypeIndexPairForControlTransfer, Descriptor,
        },
        xhci::{xhci_device::DeviceAttached, Xhci},
    },
    OsDep,
};

const TAG: &str = "[USB-HID DRIVER]";

pub struct USBDeviceDriverHidMouseExample {
    hub: usize,
    port: usize,
    slot: usize,
}

impl USBDeviceDriverHidMouseExample {
    fn operate_device<F, T, O>(&self, xhci: &Xhci<O>, mut op: F) -> T
    where
        F: Fn(&mut DeviceAttached<O>) -> T,
        O: OsDep,
    {
        op(xhci
            .dev_ctx
            .lock()
            .attached_set
            .get_mut(&(self.slot))
            .unwrap())
    }
}

impl<O> USBDeviceDriverOps<O> for USBDeviceDriverHidMouseExample
where
    O: OsDep,
{
    fn try_create(
        device: &mut DeviceAttached<O>,
    ) -> Option<alloc::sync::Arc<spinlock::SpinNoIrq<Self>>> {
        debug!("creating!");
        match {
            let fetch_desc_devices = &device.fetch_desc_devices();
            let dev_desc = fetch_desc_devices.first().unwrap();
            Some(
                if dev_desc.class
                    == USBDeviceClassCode::ReferInterfaceDescriptor
                        .to_u8()
                        .unwrap()
                {
                    device
                        .fetch_desc_interfaces()
                        .get(device.current_interface)
                        .map(|desc| {
                            (
                                desc.interface_class,
                                desc.interface_subclass,
                                desc.interface_protocol,
                            )
                        })
                        .unwrap()
                } else {
                    (dev_desc.class, dev_desc.subclass, dev_desc.protocol)
                },
            )
            .map(|(class, subclass, protocol)| {
                (
                    USBDeviceClassCode::from_u8(class),
                    USBHIDSubClassDescriptorType::from_u8(subclass),
                    protocol,
                )
            })
            .unwrap()
        } {
            (Some(USBDeviceClassCode::HID), Some(USBHIDSubClassDescriptorType::Mouse), _) => {
                Some(Arc::new(SpinNoIrq::new(Self {
                    hub: device.hub,
                    port: device.port,
                    slot: device.slot_id,
                })))
            }
            _ => None,
        }
    }

    fn work(&self, xhci: &Xhci<O>) {
        let interface_in_use = self.operate_device(xhci, |dev| {
            dev.fetch_desc_interfaces()[dev.current_interface].clone()
        });
        let idle_req = xhci.construct_no_data_transfer_req(
            0x21, //recipient:00001(interface),Type01:class,Direction:0(HostToDevice) //TODO, MAKE A Tool Module to convert type
            0x0A, //SET IDLE
            descriptors::DescriptorType::Hid.value_for_transfer_control_index((1 << 8 | 0) as u8), //duration 1, report 0: 1<<8 | 0
            interface_in_use.interface_number as u16,
            TransferType::No,
        );

        {
            debug!("{TAG}: post idle request");
            let result = self.operate_device(xhci, |dev| {
                xhci.post_control_transfer_no_data(
                    idle_req,
                    dev.transfer_rings.get_mut(0).unwrap(), //ep0 ring
                    1,                                      //to ep0
                    dev.slot_id,
                )
            });
            debug!("{TAG}: result: {:?}", result);
            // debug!("{TAG} buffer: {:?}", buffer);
        }

        debug!("waiting for event!");
        let busy_wait_for_event = xhci.busy_wait_for_event();
        debug!("getted: {:?}", busy_wait_for_event);
    }
}
