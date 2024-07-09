use core::{fmt::Error, mem::size_of, ops::DerefMut, time::Duration};

use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeSet, format, sync::Arc, vec::Vec};
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
            command::{self, ConfigureEndpoint, EvaluateContext},
            event::Allowed,
            transfer::{self, TransferType},
        },
    },
};

use crate::{
    ax::{USBDeviceDriverOps, USBHostDriverOps},
    dma::DMA,
    err::{self, Result},
    host::{
        usb::{
            define::*,
            descriptors::{
                self, desc_configuration, desc_device, desc_endpoint, desc_hid,
                desc_interface::{self, Interface},
                Descriptor, DescriptorType,
            },
        },
        Controller, ControllerArc, PortSpeed,
    },
    OsDep,
};

const TAG: &str = "[XHCI DEVICE]";

use super::{
    event::{self, Ring},
    Xhci,
};

#[derive(Debug, Default, Clone)]
pub struct DescriptorInterface {
    pub data: desc_interface::Interface,
    pub endpoints: Vec<desc_endpoint::Endpoint>,
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorConfiguration {
    pub data: desc_configuration::Configuration,
    pub interfaces: Vec<DescriptorInterface>,
}

#[derive(Clone)]
pub struct DeviceAttached<O>
where
    O: OsDep,
{
    pub hub: usize,
    pub port_id: usize,
    pub slot_id: usize,
    pub configs: Vec<DescriptorConfiguration>,
    current_config: usize,
    current_interface: usize,
    pub(crate) controller: ControllerArc<O>,
    pub device_desc: descriptors::desc_device::Device,
    os: O,
}

impl<O> DeviceAttached<O>
where
    O: OsDep,
{
    pub(crate) fn new(
        slot: usize,
        hub: usize,
        port_id: usize,
        os: O,
        controller: ControllerArc<O>,
    ) -> Self {
        Self {
            hub,
            port_id,
            slot_id: slot,
            configs: Vec::new(),
            current_config: 0,
            current_interface: 0,
            controller,
            os,
            device_desc: Default::default(),
        }
    }
    pub fn current_config(&self) -> &DescriptorConfiguration {
        &self.configs[self.current_config]
    }

    pub fn current_interface(&self) -> &DescriptorInterface {
        &self.current_config().interfaces[self.current_interface]
    }

    pub fn control_transfer_in(
        &self,
        endpoint: usize,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        len: usize,
    ) -> Result<Vec<u8>> {
        let dci;
        if endpoint == 0 {
            dci = 1;
        } else {
            if endpoint & 0x80 == 0 {
                return Err(err::Error::Param(format!("ep {endpoint:#X} not in!")));
            }
            dci = (endpoint | !0x80) * 2 + 1;
        }

        debug!("CTL ep {endpoint:#X}, dci {}", dci);

        let mut ctl = self.controller.lock();
        let transfer_type = if len > 0 {
            TransferType::In
        } else {
            TransferType::No
        };
        let buffer = if len > 0 {
            Some(DMA::new_vec(0u8, len, 64, self.os.dma_alloc()))
        } else {
            None
        };

        let data = if let Some(ref buffer) = buffer {
            let mut data = transfer::DataStage::default();
            data.set_data_buffer_pointer(buffer.addr() as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(transfer::Direction::In);
            Some(data)
        } else {
            None
        };
        let mut setup = transfer::SetupStage::default();
        setup
            .set_request_type(request_type)
            .set_request(request)
            .set_value(value)
            .set_index(index)
            .set_length(len as _)
            .set_transfer_type(transfer_type);

        debug!("{:#?}", setup);

        let mut status = transfer::StatusStage::default();

        status.set_interrupt_on_completion();

        ctl.post_transfer(setup, data, status, self, dci as _)?;

        Ok(match &buffer {
            Some(b) => b.to_vec(),
            None => Vec::new(),
        })
    }

    pub fn control_transfer_out(
        &self,
        endpoint: usize,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result {
        let len = data.len();
        let dci;
        if endpoint == 0 {
            dci = 1;
        } else {
            if endpoint & 0x80 > 0 {
                return Err(err::Error::Param(format!("ep {endpoint:#X} not out!")));
            }
            dci = endpoint * 2;
        }

        debug!("CTL ep {endpoint:#X}, dci {}", dci);

        let mut ctl = self.controller.lock();
        let transfer_type = if len > 0 {
            TransferType::Out
        } else {
            TransferType::No
        };
        let buffer = if len > 0 {
            let mut buffer = DMA::new_vec(0u8, len, 64, self.os.dma_alloc());
            buffer.copy_from_slice(data);
            Some(buffer)
        } else {
            None
        };

        let data = if let Some(ref buffer) = buffer {
            let mut data = transfer::DataStage::default();
            data.set_data_buffer_pointer(buffer.addr() as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(transfer::Direction::Out);
            Some(data)
        } else {
            None
        };
        let mut setup = transfer::SetupStage::default();
        setup
            .set_request_type(request_type)
            .set_request(request)
            .set_value(value)
            .set_index(index)
            .set_length(len as _)
            .set_transfer_type(transfer_type);

        debug!("{:#?}", setup);

        let mut status = transfer::StatusStage::default();

        status.set_interrupt_on_completion();

        ctl.post_transfer(setup, data, status, self, dci as _)?;

        Ok(())
    }
    pub fn test(&self) -> Result {
        debug!("test begin");

        let raw = self.control_transfer_in(
            0,
            ENDPOINT_IN,
            REQUEST_GET_DESCRIPTOR,
            DescriptorType::Device.forLowBit(0).bits(),
            0,
            size_of::<desc_device::Device>(),
        )?;

        if let Descriptor::Device(hid) = Descriptor::from_slice(&raw).unwrap() {
            debug!("{:#?}", hid);
        }
        debug!("post idle request to control endpoint");

        self.control_transfer_out(
            0,
            0b00100001, //recipient:00001(interface),Type01:class,Direction:0(HostToDevice) //TODO, MAKE A Tool Module to convert type
            0x0A,       //SET IDLE
            0x00,       //recommended infini idle rate for mice, refer usb Hid 1.1 spec - page 53
            // upper 8 bit = 0-> infini idle, lower 8 bit = 0-> apply to all report
            self.current_interface().data.interface_number as u16,
            &[],
        )?;
        debug!("post set protocol request");

        self.control_transfer_out(
            0,
            ENDPOINT_OUT | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
            0x0B,
            1,
            0,
            &[],
        )?;

        Ok(())
    }
}
