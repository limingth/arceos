use core::intrinsics::discriminant_value;
use core::mem::MaybeUninit;
use core::usize;

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use axhal::time::busy_wait_until;
use log::*;
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
use crate::usb::trasnfer::control::{
    bmRequestType, ControlTransfer, DataTransferType, Recipient, StandardbRequest,
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

pub fn init_ch341(&mut self) -> bool{
    trace!("hid mouse preparing for drive!");
    let endpoint_in = self.interrupt_in_channels.last().unwrap();
    let mut todo_list = Vec::new();
    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::In,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: StandardbRequest::SetConfiguration.into(),
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
            request: StandardbRequest::SetConfiguration.into(),
            index: 0 as u16,
            value: 0 as u16,
            data: None,
        }),
    ));
    if !SetBaudRate(&mut self, 9600){
        return false;
    }
    if !SetLineProperties(&mut self, 8, 0, 1){
        return false;
    }
    return true;
}

fn SetBaudRate(&mut self,rate:usize) -> bool{
    let factor:u32 = 1532620800/rate;
    let divisor:u16 = 3;
    while (factor > 0xfff0) && divisor {
        factor >>= 3;
        divisor-=1;
    }
    if factor > 0xfff0{
        trace!("factor wrror");
        return false;
    }
    factor = 0x10000 - factor;
    let a:u16 = (factor & 0xff00) | divisor;
    a |= 1 << 7;
    let endpoint_in = self.interrupt_in_channels.last().unwrap();
    let mut todo_list = Vec::new();
    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::Out,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: StandardbRequest::SetConfiguration.into(),
            index: 0x1312 as u16,
            value: 0 as u16,
            data: None,
        }),
    ));
    return true;
}

fn SetLineProperties(&mut self,nDataBits:u8,nParity:u8,nStopBits:u8) -> bool{
    let buffer:Vec<u8>;
    let lcr:u8 = 0x80|0x40;
    match nDataBits{
        5 => lcr |= 0x00,
        6 => lcr |= 0x01,
        7 => lcr |= 0x02,
        8 => lcr |= 0x03,
        _ => {trace!("Invalid data bits {:?}", nDataBits);
            return false;
        },
    }

    match nParity{
        0 => buffer.append("N"),
        1 => {lcr |= 0x08;
            buffer.append("O");
        },
        2 => {lcr |= 0x80 | 0x10;
            buffer.append("E");
        },
        _ => trace!("Invalid parity {:?}",nParity),
    }

    match nStopBits{
        1 => buffer.append("1"),
        2 => {lcr |= 0x04;
            buffer.append("2");
        },
        _ => {trace!("Invalid stop bits {:?}", nStopBits);
            return false;
        },
    }
    let mcr:u8 = 0;
    let endpoint_in = self.interrupt_in_channels.last().unwrap();
    let mut todo_list = Vec::new();
    todo_list.push(URB::new(
        self.device_slot_id,
        RequestedOperation::Control(ControlTransfer {
            request_type: bmRequestType::new(
                Direction::Out,
                DataTransferType::Vendor,
                Recipient::Device,
            ),
            request: StandardbRequest::SetConfiguration.into(),
            index: !mcr as u16,
            value: 0 as u16,
            data: None,
        }),
    ));
    return true;
}




