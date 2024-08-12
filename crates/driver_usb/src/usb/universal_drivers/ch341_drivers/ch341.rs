use core::mem::MaybeUninit;

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use log::trace;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::context::EndpointType;
use xhci::ring::trb::transfer::Direction;

use crate::abstractions::dma::DMA;
use crate::glue::ucb::{TransferEventCompleteCode, UCB};
use crate::usb::descriptors::desc_hid::HIDDescriptorTypes;
use crate::usb::descriptors::topological_desc::{
    TopologicalUSBDescriptorEndpoint, TopologicalUSBDescriptorFunction,
};
use crate::usb::descriptors::USBStandardDescriptorTypes;
use crate::usb::operation::ExtraStep;
use crate::usb::trasnfer::bulk::BulkTransfer;
use crate::usb::trasnfer::control::{
    bmRequestType, ControlTransfer, DataTransferType, Recipient, StandardbRequest,bRequest
};
use crate::usb::trasnfer::interrupt::InterruptTransfer;
use crate::usb::universal_drivers::BasicSendReceiveStateMachine;
use crate::usb::urb::{RequestedOperation, URB};
use crate::USBSystemConfig;
use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{desc_device::StandardUSBDeviceClassCode, desc_endpoint::Endpoint},
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    },
};

pub struct CH341driverModule;

impl<'a, O> USBSystemDriverModule<'a, O> for CH341driverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &mut DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(desc) = &*independent_dev.descriptors {
            if desc.device.iter().any(|desc| desc.data.class == 255) {
                if let Some(device1) = desc.device.get(0) {
                    if let Some(child1) = device1.child.get(0) {
                        if let Some(child2) = child1.child.get(0) {
                            match child2 {
                                TopologicalUSBDescriptorFunction::Interface(interface_data) => {
                                    for (interface, usb_descriptors, endpoints) in interface_data {
                                        if interface.interface_class == 255 {
                                            return Some(
                                                (vec![CH341driver::new_and_init(
                                                    independent_dev.slotid,
                                                    device1.data.protocol,
                                                    {
                                                        device1
                                                        .child
                                                        .iter()
                                                        .find(|c| {
                                                            c.data.config_val() == independent_dev.configuration_val as u8
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
                                                                            == independent_dev.interface_val as u8
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
                                                        .filter_map(|e| {
                                                            if let TopologicalUSBDescriptorEndpoint::Standard(ep) = e {
                                                                Some(ep)
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .collect()
                                                    },
                                                    config.clone(),
                                                    independent_dev.interface_val,
                                                    independent_dev
                                                        .current_alternative_interface_value,
                                                    independent_dev.configuration_val,
                                                )]),
                                            );
                                        };
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn preload_module(&self) {
        trace!("nothing");
    }
}

pub struct CH341driver<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    bootable: usize,
    device_slot_id: usize,
    interrupt_in_channels: Vec<u32>,
    interrupt_out_channels: Vec<u32>,
    bulk_in_channels: Vec<u32>,
    bulk_out_channels: Vec<u32>,
    interface_value: usize, //temporary place them here
    interface_alternative_value: usize,
    config_value: usize, // same
    driver_state_machine: BasicSendReceiveStateMachine,
    receiption_buffer: Option<SpinNoIrq<DMA<[u8], O::DMA>>>,
}

impl<'a, O> CH341driver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn new_and_init(
        device_slot_id: usize,
        bootable: u8,
        endpoints: Vec<Endpoint>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        interface_value: usize,
        alternative_val: usize,
        config_value: usize,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        Arc::new(SpinNoIrq::new(Self {
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
            bulk_in_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::BulkIn => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            bulk_out_channels: {
                endpoints
                    .iter()
                    .filter_map(|ep| match ep.endpoint_type() {
                        EndpointType::InterruptOut => Some(ep.doorbell_value_aka_dci()),
                        _ => None,
                    })
                    .collect()
            },
            config,
            interface_value,
            config_value,
            interface_alternative_value: alternative_val,
            bootable: bootable as usize,
            driver_state_machine: BasicSendReceiveStateMachine::Sending,
            receiption_buffer: None,
        }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for CH341driver<O>
where
    O: PlatformAbstractions,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<URB<'a, O>>> {
        let last = self.interrupt_in_channels.last().unwrap();
        let endpoint_in = last;
        let mut todo_list = Vec::new();
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::In,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::DriverSpec(0x5F),
                index: 0 as u16,
                value: 0 as u16,
                data: None,
            }),
        ));
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::DriverSpec(0xA1),
                index: 0 as u16,
                value: 0 as u16,
                data: None,
            }),
        ));
        let mut rate: usize = 9600;
        let mut lcr: u8 = 0x80 | 0x40 | 0x03;
        let mut factor: u32 = (1532620800 / rate).try_into().unwrap();
        let mut divisor: u16 = 3;
        while (factor > 0xfff0) && (divisor > 0) {
            factor >>= 3;
            divisor -= 1;
        }
        if factor > 0xfff0 {
            trace!("factor wrror");
        }
        factor = 0x10000 - factor;
        let mut a: u16 = (factor & 0xff00) as u16| divisor;
        a |= 1 << 7;

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::DriverSpec(0x9A),
                index: a as u16,
                value: 0x1312 as u16,
                data: None,
            }),
        ));

        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::DriverSpec(0x9A),
                index: lcr as u16,
                value: 0x2518 as u16,
                data: None,
            }),
        ));
        let mcr = 0;
        todo_list.push(URB::new(
            self.device_slot_id,
            RequestedOperation::Control(ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Vendor,
                    Recipient::Device,
                ),
                request: bRequest::DriverSpec(0xA4),
                index: !mcr as u16,
                value: 0 as u16,
                data: None,
            }),
        ));
        Some(todo_list)
        //////////////////////////
    }

    fn gather_urb(&mut self) -> Option<Vec<URB<'a, O>>> {
        match self.driver_state_machine {
            BasicSendReceiveStateMachine::Waiting => None,
            BasicSendReceiveStateMachine::Sending => {
                self.driver_state_machine = BasicSendReceiveStateMachine::Waiting;
                match &self.receiption_buffer {
                    Some(buffer) => buffer.lock().fill_with(|| 0u8),
                    None => {
                        self.receiption_buffer = Some(SpinNoIrq::new(DMA::new_vec(
                            0u8,
                            8,
                            O::PAGE_SIZE,
                            self.config.lock().os.dma_alloc(),
                        )))
                    }
                }

                if let Some(buffer) = &mut self.receiption_buffer {
                    trace!("some!");
                    return Some(vec![URB::<O>::new(
                        self.device_slot_id,
                        RequestedOperation::Interrupt(InterruptTransfer {
                            endpoint_id: self.interrupt_in_channels.last().unwrap().clone()
                                as usize,
                            buffer_addr_len: buffer.lock().addr_len_tuple(),
                        }),
                    )]);
                }
                None
            }
        }
    }

    fn receive_complete_event(&mut self, ucb: UCB<O>) {
        match ucb.code {
            crate::glue::ucb::CompleteCode::Event(TransferEventCompleteCode::Success) => {
                trace!("completed!");
                self.receiption_buffer
                    .as_ref()
                    .map(|a| a.lock().to_vec().clone())
                    .inspect(|a| {
                        trace!("current buffer:{:?}", a);
                    });
                self.driver_state_machine = BasicSendReceiveStateMachine::Sending
            }
            other => panic!("received {:?}", other),
        }
    }
}

pub fn ch341_open(){
    
}

pub fn ch341_set_termios(nDataBits:u8,nParity:u8,nStopBits:u8) {
    let baud_rate:usize = 9600;
    let mut flags = 0u64;
    let mut lcr: u8;

    let mut factor: u32 = (1532620800 / baud_rate).try_into().unwrap();
    let mut divisor: u16 = 3;
    while (factor > 0xfff0) && (divisor > 0) {
        factor >>= 3;
        divisor -= 1;
    }
    if factor > 0xfff0 {
        trace!("factor wrror");
    }
    factor = 0x10000 - factor;
    let mut a: u16 = (factor & 0xff00) as u16| divisor;
    a |= 1 << 7;

    let endpoint_in = last;
    let mut todo_list = Vec::new();

    lcr = 0x80 | 0x40;

    match nDataBits {
        5 => lcr |= 0x00,
        6 => lcr |= 0x01,
        7 => lcr |= 0x02,
        8 => lcr |= 0x03,
        _ => (),
    }

    if C_PARENB(tty) {
        lcr |= 0x08;
        if C_PARODD(tty) == 0 {
            lcr |= 0x10;
        }
        if C_CMSPAR(tty) {
            lcr |= 0x20;
        }
    }

    if C_CSTOPB(tty) {
        lcr |= 0x04;
    }

    //ch341_set_baudrate_lcr(port.serial.dev, priv, priv.baud_rate, lcr);
    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::Out,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: bRequest::DriverSpec(0x9A),
            index: a as u16,
            value: 0x1312 as u16,
            data: None,
        }),
    ));

    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::Out,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: bRequest::DriverSpec(0x9A),
            index: lcr as u16,
            value: 0x2518 as u16,
            data: None,
        }),
    ));

    let mut mcr = 0;
    mcr |= ((1 << 5)|(1 << 6));
    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::Out,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: bRequest::DriverSpec(0xA4),
            index: !mcr as u16,
            value: 0 as u16,
            data: None,
        }),
    ));
}
