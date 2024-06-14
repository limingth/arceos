use core::{marker::PhantomData, time::Duration};

use alloc::format;
use alloc::{collections::BTreeMap, fmt::format, string::String, sync::Arc};
use axhal::{paging::PageSize, time::busy_wait};
use axtask::sleep_until;
use driver_common::BaseDriverOps;
use log::debug;
use num_traits::{FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::ring::trb::transfer::{self, Direction, TransferType};

use crate::{
    ax::USBDeviceDriverOps,
    dma::DMA,
    host::{
        usb::descriptors::{
            self,
            desc_device::{Device, USBDeviceClassCode},
            desc_hid::{Hid, USBHIDProtocolDescriptorType, USBHIDSubclassDescriptorType},
            DescriptionTypeIndexPairForControlTransfer, Descriptor, DescriptorType,
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
    hid_desc: Hid,
}

impl USBDeviceDriverHidMouseExample {
    fn operate_device<F, T, O>(&self, xhci: &Xhci<O>, mut op: F) -> T
    where
        F: FnMut(&mut DeviceAttached<O>) -> T,
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
        let fetch_desc_hid = &device.fetch_desc_hid();
        match {
            let fetch_desc_devices = &device.fetch_desc_devices();
            debug!("desc_device: {:?}", fetch_desc_devices);
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
                debug!("interface csp:{class},{subclass},{protocol}");
                (
                    USBDeviceClassCode::from_u8(class),
                    USBHIDSubclassDescriptorType::from_u8(subclass),
                    USBHIDProtocolDescriptorType::from_u8(protocol),
                )
            })
            .unwrap()
        } {
            (Some(USBDeviceClassCode::HID), Some(_), Some(USBHIDProtocolDescriptorType::Mouse)) => {
                Some(Arc::new(SpinNoIrq::new(Self {
                    hub: device.hub,
                    port: device.port,
                    slot: device.slot_id,
                    hid_desc: fetch_desc_hid[0].clone(),
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
            descriptors::DescriptorType::Hid.forLowBit((1 << 8 | 0) as u8), //duration 1, report 0: 1<<8 | 0
            interface_in_use.interface_number as u16,
            TransferType::No,
        );

        {
            //set idle
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

        {
            busy_wait(Duration::from_millis(500));
            //request report rate
            let buffer = DMA::new_vec(
                0u8,
                self.hid_desc.report_descriptor_len as usize,
                64,
                xhci.config.os.dma_alloc(),
            );
            let request_report = xhci.construct_control_transfer_req(
                &buffer,
                0x81, //recipient:00001(interface),Type00:standard,Direction:01(DeviceToHost) //TODO, MAKE A Tool Module to convert type
                0x06, //get descriptor
                DescriptorType::HIDReport.forLowBit(0), //report descriptor
                0,    //interface
                (TransferType::In, Direction::In),
            );

            debug!("{TAG}: post report request");
            let result = self
                .operate_device(xhci, |dev| {
                    xhci.post_control_transfer_with_data_and_busy_wait(
                        request_report,
                        dev.transfer_rings.get_mut(0).unwrap(), //ep0 ring
                        1,                                      //to ep0
                        dev.slot_id,
                    )
                })
                .unwrap();
            debug!("{TAG}: result: {:?}", result);
            print_array(&buffer);

            // ReportHandler::new(&buffer).unwrap()
        } //TODO parse Report context

        loop {
            busy_wait(Duration::from_secs(1)); //too high, just for debug

            // loop { //TODO: check endpoint state to ensure data commit complete
            //     busy_wait(Duration::from_millis(10));
            //     xhci.regs
            //         .lock()
            //         .regs
            //         .port_register_set
            //         .read_volatile_at(self.port)
            //         .portsc
            //         .port_state
            // }

            self.operate_device(xhci, |dev| {
                //get input endpoint dci, we only pick endpoint in #0 here
                dev.operate_endpoint_in(|mut endpoints, rings| {
                    let in_dci = endpoints.get_mut(0).unwrap().doorbell_value_aka_dci();
                    let buffer = DMA::new_vec(0u8, 8, 64, xhci.config.os.dma_alloc()); //enough for a mouse Report(should get from report above,but we not parse it yet)

                    let req = {
                        //for a interrupt transfer, it didn't invove setup stage.
                        let data = *transfer::DataStage::default()
                            .set_data_buffer_pointer(buffer.addr() as u64)
                            .set_direction(Direction::In);

                        let status =
                            *transfer::StatusStage::default().set_interrupt_on_completion();

                        (data.into(), status.into())
                    };

                    debug!("{TAG}: post report request");
                    let result = self.operate_device(xhci, |dev| {
                        xhci.post_transfer_not_control(
                            req,
                            rings.get_mut(in_dci as usize).unwrap(), //ep0 ring
                            in_dci as u8,                            //to ep0
                            dev.slot_id,
                        )
                    });
                    debug!("{TAG}: result: {:?}", result);
                    print_array(&buffer);
                });
            })
        }
    }
}

fn print_array(arr: &[u8]) {
    let mut line = String::new();
    for (i, &byte) in arr.iter().enumerate() {
        line.push_str(&format!("{:02x} ", byte));
        if (i + 1) % 4 == 0 {
            debug!("{}", line);
            line.clear();
        }
    }
    if !line.is_empty() {
        debug!("{}", line);
    }
}

struct USBHIDReportMouse {}
