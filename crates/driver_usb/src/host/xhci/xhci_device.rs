use core::{fmt::Error, ops::DerefMut, time::Duration};

use alloc::{borrow::ToOwned, collections::BTreeSet, sync::Arc, vec::Vec};
use axhal::time::busy_wait_until;
use axtask::sleep;
use log::debug;
use num_derive::FromPrimitive;
use num_traits::{ops::mul_add, FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::{
    context::{Endpoint, EndpointType, Input64Byte, InputHandler},
    ring::{
        self,
        trb::{
            command::{self, Allowed, ConfigureEndpoint},
            transfer,
        },
    },
};

use crate::{
    ax::{USBDeviceDriverOps, USBHostDriverOps},
    dma::DMA,
    err::{self, Result},
    host::{
        usb::descriptors::{self, desc_interface::Interface, Descriptor},
        PortSpeed,
    },
    OsDep,
};

const TAG: &str = "[XHCI DEVICE]";

use super::{
    event::{self, Ring},
    Xhci,
};

pub struct DeviceAttached<O>
where
    O: OsDep,
{
    pub hub: usize,
    pub port: usize,
    pub num_endp: usize,
    pub slot_id: usize,
    pub transfer_rings: Vec<Ring<O>>,
    pub descriptors: Vec<descriptors::Descriptor>,
    pub current_interface: usize,
}

impl<O> DeviceAttached<O>
where
    O: OsDep,
{
    pub fn find_driver_impl<T: USBDeviceDriverOps<O>>(&mut self) -> Option<Arc<SpinNoIrq<T>>> {
        // let device = self.fetch_desc_devices()[0]; //only pick first device desc
        debug!("try creating!");
        T::try_create(self)
    }

    pub fn set_configuration<FC, FT>(
        &mut self,
        port_speed: PortSpeed,
        mut post_cmd: FC,
        mut post_transfer: FT,
        input_ref: &mut Vec<DMA<Input64Byte, O::DMA>>,
    ) where
        FC: FnMut(command::Allowed) -> Result<ring::trb::event::CommandCompletion>,
        FT: FnMut(
            (transfer::Allowed, transfer::Allowed, transfer::Allowed), //setup,data,status
            &mut Ring<O>,                                              //transfer ring
            u8,                                                        //dci
            usize,                                                     //slot
        ) -> Result<ring::trb::event::TransferEvent>,
    {
        let last_entry = self
            .fetch_desc_endpoints()
            .iter()
            .max_by_key(|e| e.doorbell_value_aka_dci())
            .unwrap()
            .to_owned();

        debug!("found last entry: {:?}", last_entry.endpoint_address);

        let input = input_ref.get_mut(self.slot_id).unwrap().deref_mut();
        let slot_mut = input.device_mut().slot_mut();
        slot_mut.set_context_entries(last_entry.doorbell_value_aka_dci() as u8);

        let control_mut = input.control_mut();

        let interface = self.fetch_desc_interfaces()[0].clone(); //hardcoded 0 interface
        self.current_interface = 0;
        control_mut.set_interface_number(interface.interface_number);
        control_mut.set_alternate_setting(interface.alternate_setting);

        control_mut.set_add_context_flag(1);
        // control_mut.set_drop_context_flag(0);
        // always choose last config here(always only 1 config exist, we assume.), need to change at future
        control_mut.set_configuration_value(self.fetch_desc_configs()[0].config_val());

        self.fetch_desc_endpoints().iter().for_each(|ep| {
            self.init_endpoint_context(port_speed, ep, input);
        });

        debug!("{TAG} CMD: configure endpoint");
        let post_cmd = post_cmd(Allowed::ConfigureEndpoint(
            *ConfigureEndpoint::default()
                .set_slot_id(self.slot_id as u8)
                .set_input_context_pointer((input as *mut Input64Byte).addr() as u64),
        ));
        debug!("{TAG} CMD: result:{:?}", post_cmd);
    }

    fn init_endpoint_context(
        &self,
        port_speed: PortSpeed,
        endpoint_desc: &descriptors::desc_endpoint::Endpoint,
        input_ctx: &mut Input64Byte,
    ) {
        //set add content flag
        let control_mut = input_ctx.control_mut();
        control_mut.set_add_context_flag(0);
        control_mut.clear_add_context_flag(1); // See xHCI dev manual 4.6.6.
        control_mut.add_context_flag(endpoint_desc.doorbell_value_aka_dci() as usize);

        let endpoint_mut = input_ctx
            .device_mut()
            .endpoint_mut(endpoint_desc.doorbell_value_aka_dci() as usize);
        //set interval
        // let port_speed = PortSpeed::get(port_number);
        let endpoint_type = endpoint_desc.endpoint_type();
        let interval = endpoint_desc.calc_actual_interval(port_speed);

        endpoint_mut.set_interval(interval);

        //init endpoint type
        let endpoint_type = endpoint_desc.endpoint_type();
        endpoint_mut.set_endpoint_type(endpoint_type);

        {
            let max_packet_size = endpoint_desc.max_packet_size;
            let ring_addr = self
                .transfer_rings
                .get(endpoint_desc.doorbell_value_aka_dci() as usize)
                .unwrap()
                .register();
            match endpoint_type {
                EndpointType::Control => {
                    endpoint_mut.set_max_packet_size(max_packet_size);
                    endpoint_mut.set_error_count(3);
                    endpoint_mut.set_tr_dequeue_pointer(ring_addr);
                    endpoint_mut.set_dequeue_cycle_state();
                }
                EndpointType::BulkOut | EndpointType::BulkIn => {
                    endpoint_mut.set_max_packet_size(max_packet_size);
                    endpoint_mut.set_max_burst_size(0);
                    endpoint_mut.set_error_count(3);
                    endpoint_mut.set_max_primary_streams(0);
                    endpoint_mut.set_tr_dequeue_pointer(ring_addr);
                    endpoint_mut.set_dequeue_cycle_state();
                }
                EndpointType::IsochOut
                | EndpointType::IsochIn
                | EndpointType::InterruptOut
                | EndpointType::InterruptIn => {
                    //init for isoch/interrupt
                    endpoint_mut.set_max_packet_size(max_packet_size & 0x7ff); //wtf
                    endpoint_mut
                        .set_max_burst_size(((max_packet_size & 0x1800) >> 11).try_into().unwrap());
                    endpoint_mut.set_mult(0);

                    if let EndpointType::IsochOut | EndpointType::IsochIn = endpoint_type {
                        endpoint_mut.set_error_count(0);
                    } else {
                        endpoint_mut.set_error_count(3);
                    }

                    endpoint_mut.set_tr_dequeue_pointer(ring_addr);
                    endpoint_mut.set_dequeue_cycle_state();
                }
                EndpointType::NotValid => unreachable!("Not Valid Endpoint should not exist."),
            }
        }
    }

    //consider use marcos to these bunch of methods
    pub fn fetch_desc_configs(&mut self) -> Vec<descriptors::desc_configuration::Configuration> {
        self.descriptors
            .iter()
            .filter_map(|desc| match desc {
                Descriptor::Configuration(config) => Some(config.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn fetch_desc_hid(&mut self) -> Vec<descriptors::desc_hid::Hid> {
        self.descriptors
            .iter()
            .filter_map(|desc| match desc {
                Descriptor::Hid(hid) => Some(hid.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn fetch_desc_devices(&mut self) -> Vec<descriptors::desc_device::Device> {
        self.descriptors
            .iter()
            .filter_map(|desc| match desc {
                Descriptor::Device(device) => Some(device.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn has_desc<F>(&mut self, predicate: F) -> bool
    where
        F: FnMut(&Descriptor) -> bool,
    {
        self.descriptors.iter().any(predicate)
    }

    pub fn fetch_desc_interfaces(&mut self) -> Vec<descriptors::desc_interface::Interface> {
        self.descriptors
            .iter()
            .filter_map(|desc| {
                if let Descriptor::Interface(int) = desc {
                    Some(int.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn fetch_desc_endpoints(&mut self) -> Vec<descriptors::desc_endpoint::Endpoint> {
        self.descriptors
            .iter()
            .filter_map(|desc| {
                if let Descriptor::Endpoint(e) = desc {
                    Some(e.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn operate_endpoint_in<R, F>(&mut self, mapper: F) -> R
    where
        F: Fn(Vec<&descriptors::desc_endpoint::Endpoint>, &mut Vec<Ring<O>>) -> R,
    {
        mapper(
            self.fetch_desc_endpoints()
                .iter()
                .filter(|endpoint| endpoint.endpoint_type() == EndpointType::InterruptIn)
                .collect(),
            &mut self.transfer_rings,
        )
    }
}
