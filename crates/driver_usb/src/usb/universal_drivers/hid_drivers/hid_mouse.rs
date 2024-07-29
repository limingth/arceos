use alloc::vec;
use alloc::vec::Vec;
use driver_common::DeviceType;
use log::trace;
use num_traits::FromPrimitive;
use xhci::context::EndpointType;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            desc_device::USBDeviceClassCode, desc_endpoint::Endpoint,
            TopologicalUSBDescriptorFunction,
        },
        drivers::driverapi::USBSystemDriverModule,
    },
};

use super::USBHidDeviceSubClassCode;

#[derive(Debug)]
struct HidMouseDriver {
    device_slot_id: usize,
    interrupt_in_channels: Vec<u32>,
    interrupt_out_channels: Vec<u32>,
}

impl HidMouseDriver {
    fn new_and_init(device_slot_id: usize, bootable: u8, endpoints: &Vec<Endpoint>) -> Self {
        Self {
            device_slot_id,
            interrupt_in_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptIn => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            interrupt_out_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptOut => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
        }
    }
}

impl<'a, O> USBSystemDriverModule<'a, O> for HidMouseDriver
where
    O: PlatformAbstractions,
{
    fn should_active(independent_dev: DriverIndependentDeviceInstance<O>) -> Option<Vec<Self>> {
        if let MightBeInited::Inited(inited) = independent_dev.descriptors {
            let device = inited.device.first().unwrap();
            return match (
                USBDeviceClassCode::from_u8(device.data.class),
                USBHidDeviceSubClassCode::from_u8(device.data.subclass),
                device.data.protocol,
            ) {
                (
                    Some(USBDeviceClassCode::HID),
                    Some(USBHidDeviceSubClassCode::Mouse),
                    bootable,
                ) => {
                    return Some(vec![Self::new_and_init(
                        independent_dev.slotid,
                        bootable,
                        &{
                            device
                                .child
                                .iter()
                                .find(|c| {
                                    c.data.config_val() == independent_dev.configuration_id as u8
                                })
                                .expect("configuration not found")
                                .child
                                .iter()
                                .filter_map(|func| match func {
                                    TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                                        panic!("a super complex device, help meeeeeeeee!");
                                    }
                                    TopologicalUSBDescriptorFunction::Interface(interface) => Some(
                                        interface
                                            .iter()
                                            .find(|(interface, alternatives, endpoints)| {
                                                interface.interface_number
                                                    == independent_dev.interface_id as u8
                                                    && interface.alternate_setting
                                                        == independent_dev
                                                            .current_alternative_interface_value
                                                            as u8
                                            })
                                            .expect("invalid interface value or alternative value")
                                            .2
                                            .clone(),
                                    ),
                                })
                                .take(1)
                                .flat_map(|a| a)
                                .collect()
                        },
                    )]);
                }
                (Some(USBDeviceClassCode::ApplicationSpecific), _, _) => Some({
                    let collect = device
                        .child
                        .iter()
                        .find(|configuration| {
                            configuration.data.config_val()
                                == independent_dev.configuration_id as u8
                        })
                        .expect("configuration not found")
                        .child
                        .iter()
                        .filter_map(|interface| match interface {
                            TopologicalUSBDescriptorFunction::InterfaceAssociation((
                                asso,
                                interfaces,
                            )) if let (
                                USBDeviceClassCode::HID,
                                USBHidDeviceSubClassCode::Mouse,
                                bootable,
                            ) = (
                                USBDeviceClassCode::from_u8(asso.function_class).unwrap(),
                                USBHidDeviceSubClassCode::from_u8(asso.function_subclass).unwrap(),
                                asso.function_protocol,
                            ) =>
                            {
                                // return Some(Self::new_and_init(independent_dev.slotid, bootable));
                                panic!("a super complex device, help meeeeeeeee!");
                            }
                            TopologicalUSBDescriptorFunction::Interface(interfaces) => {
                                let (interface, additional, endpoints) = interfaces
                                    .get(independent_dev.current_alternative_interface_value)
                                    .expect("invalid anternative interface value");
                                if let (
                                    Some(USBDeviceClassCode::HID),
                                    Some(USBHidDeviceSubClassCode::Mouse),
                                    bootable,
                                ) = (
                                    USBDeviceClassCode::from_u8(interface.interface_class),
                                    USBHidDeviceSubClassCode::from_u8(interface.interface_subclass),
                                    interface.interface_protocol,
                                ) {
                                    return Some(Self::new_and_init(
                                        independent_dev.slotid,
                                        bootable,
                                        endpoints,
                                    ));
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        })
                        .collect();
                    collect
                }),
                _ => None,
            };
        }
        None
    }

    fn preload_module(&self) {
        todo!()
    }

    fn gather_urb(self: &Self) -> Option<crate::usb::urb::URB<'a, O>> {
        None
    }
}
