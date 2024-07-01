use core::{marker::PhantomData, time::Duration};

use alloc::{collections::BTreeMap, fmt::format, string::String, sync::Arc};
use alloc::{format, vec};
use axhal::{paging::PageSize, time::busy_wait};
use axtask::sleep_until;
use driver_common::BaseDriverOps;
use log::debug;
use num_traits::{FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::ring::trb::command;
use xhci::ring::trb::transfer::{self, Direction, Normal, TransferType};

use crate::host::xhci::ring::Ring;
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
            0b00100001, //recipient:00001(interface),Type01:class,Direction:0(HostToDevice) //TODO, MAKE A Tool Module to convert type
            0x0A,       //SET IDLE
            0x00,       //recommended infini idle rate for mice, refer usb Hid 1.1 spec - page 53
            // upper 8 bit = 0-> infini idle, lower 8 bit = 0-> apply to all report
            interface_in_use.interface_number as u16,
            TransferType::No, //no data applied
        );

        {
            //set idle
            debug!("{TAG}: post idle request to control endpoint");
            let result = self.operate_device(xhci, |dev| {
                xhci.post_control_transfer_no_data_and_busy_wait(
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

        // loop {
        // busy_wait(Duration::from_millis(500)); //too slow, just for debug

        // loop {} //TODO: check endpoint state to ensure data commit complete

        self.operate_device(xhci, |dev| {
            let slot_id = dev.slot_id;
            //get input endpoint dci, we only pick endpoint in #0 here
            dev.operate_endpoint_in(|mut endpoints, rings| {
                let in_dci = endpoints.get_mut(0).unwrap().doorbell_value_aka_dci(); //we use first in interrupt endpoint here, in actual environment, there might has multiple.
                let buffer = DMA::new_vec(0u8, 4, 32, xhci.config.os.dma_alloc()); //enough for a mouse Report(should get from report above,but we not parse it yet)

                debug!("{TAG}: post IN Transfer report request");
                let result = {
                    //temporary inlined, hass to be packed in to a function future
                    let this = &xhci;
                    let request = transfer::Allowed::Normal(
                        // just use normal trb to request interrupt transfer
                        *Normal::default()
                            .set_data_buffer_pointer(buffer.addr() as u64)
                            .set_td_size(0)
                            .set_trb_transfer_length(buffer.length_for_bytes() as u32)
                            .clear_interrupt_on_short_packet()
                            .clear_interrupt_on_completion(),
                    );
                    let mut transfer_rings = rings.get_many_mut([3]).unwrap(); //chaos!

                    let dci = 3 as u8;

                    {
                        let this = &this;
                        let mut transfer_trbs = vec![request];
                        transfer_rings.iter_mut().for_each(|t| {
                            t.enque_trbs(
                                transfer_trbs
                                    .iter_mut()
                                    .map(|trb| {
                                        if this.ring.lock().cycle {
                                            trb.set_cycle_bit();
                                        } else {
                                            trb.clear_cycle_bit();
                                        }
                                        trb.into_raw()
                                    })
                                    .collect(),
                            )
                        });

                        debug!("{TAG} Post control transfer! at slot_id:{slot_id},dci:{dci}");

                        let mut regs = this.regs.lock();

                        regs.regs.doorbell.update_volatile_at(slot_id, |r| {
                            r.set_doorbell_target(dci);
                        });

                        O::force_sync_cache();

                        this.busy_wait_for_event()
                    }
                };
                // busy_wait(Duration::from_millis(5));
                debug!("{TAG}: result: {:?}", result);
                print_array(&buffer);
            });
        })
        // }
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
