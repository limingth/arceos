use core::{fmt::Error, ops::DerefMut, time::Duration};

use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeSet, sync::Arc, vec::Vec};
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
        usb::descriptors::{
            self, desc_configuration, desc_endpoint,
            desc_interface::{self, Interface},
            Descriptor,
        },
        Controller, PortSpeed,
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
    pub endpoints:Vec< desc_endpoint::Endpoint>,
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorConfiguration {
    pub data: desc_configuration::Configuration,
    pub interfaces: Vec<DescriptorInterface>,
}

pub struct DeviceAttached<O>
where
    O: OsDep,
{
    pub hub: usize,
    pub port_id: usize,
    pub num_endp: usize,
    pub slot_id: usize,
    pub configs: Vec<DescriptorConfiguration>,
    pub current_interface: usize,
    pub(crate) controller: Arc<SpinNoIrq<Box<dyn Controller<O>>>>,
    pub device_desc: descriptors::desc_device::Device,
}

impl<O> DeviceAttached<O> where O: OsDep {}
