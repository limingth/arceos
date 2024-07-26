use core::ptr;

use alloc::{collections, vec, vec::Vec};
use desc_configuration::Configuration;
use desc_device::Device;
use desc_endpoint::Endpoint;
use desc_hid::Hid;
use desc_interface::{Interface, InterfaceAssociation};
use desc_str::Str;
use log::{debug, trace, warn};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use tock_registers::interfaces;

use crate::abstractions::{dma::DMA, PlatformAbstractions};

pub mod desc_configuration;
pub mod desc_device;
pub mod desc_endpoint;
pub mod desc_hid;
pub mod desc_interface;
pub mod desc_str;

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum DescriptorType {
    //USB 1.1: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    // USB 2.0: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower1 = 8,
    Hid = 0x21,
    HIDReport = 0x22,
    HIDPhysical = 0x23,
    // USB 3.0+: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    OTG = 0x09,
    Debug = 0x0a,
    InterfaceAssociation = 0x0b,
    Bos = 0x0f,
    DeviceCapability = 0x10,
    SuperSpeedEndpointCompanion = 0x30,
    SuperSpeedPlusIsochEndpointCompanion = 0x31,
}
impl DescriptorType {
    pub(crate) fn forLowBit(self, index: u8) -> DescriptionTypeIndexPairForControlTransfer {
        DescriptionTypeIndexPairForControlTransfer { ty: self, i: index }
    }
}
pub(crate) struct DescriptionTypeIndexPairForControlTransfer {
    ty: DescriptorType,
    i: u8,
}

impl DescriptionTypeIndexPairForControlTransfer {
    pub(crate) fn bits(self) -> u16 {
        (self.ty as u16) << 8 | u16::from(self.i)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum USBDescriptor {
    Device(Device),
    Configuration(Configuration),
    Str(Str),
    Interface(Interface),
    InterfaceAssociation(InterfaceAssociation),
    Endpoint(Endpoint),
    Hid(Hid),
}

pub(crate) struct RawDescriptorParser<O: PlatformAbstractions> {
    raw: DMA<[u8], O::DMA>,
    current: usize,
    len: usize,
}

#[derive(Debug)]
pub(crate) enum Error {
    UnrecognizedType(u8),
    ParseOrderError,
    EndOfDescriptors,
}

impl USBDescriptor {
    pub(crate) fn from_slice(raw: &[u8]) -> Result<Self, Error> {
        assert_eq!(raw.len(), raw[0].into());
        match FromPrimitive::from_u8(raw[1]) {
            Some(t) => {
                let raw: *const [u8] = raw;
                match t {
                    // SAFETY: This operation is safe because the length of `raw` is equivalent to the
                    // one of the descriptor.
                    DescriptorType::Device => Ok(Self::Device(unsafe { ptr::read(raw.cast()) })),
                    DescriptorType::Configuration => {
                        Ok(Self::Configuration(unsafe { ptr::read(raw.cast()) }))
                    }
                    DescriptorType::String => Ok(Self::Str(unsafe { ptr::read(raw.cast()) })),
                    DescriptorType::Interface => {
                        Ok(Self::Interface(unsafe { ptr::read(raw.cast()) }))
                    }
                    DescriptorType::Endpoint => {
                        Ok(Self::Endpoint(unsafe { ptr::read(raw.cast()) }))
                    }
                    DescriptorType::Hid => Ok(Self::Hid(unsafe { ptr::read(raw.cast()) })),
                    other => unimplemented!("please implement descriptor type:{:?}", other),
                }
            }
            None => {
                trace!("unrecognized type!");
                Err(Error::UnrecognizedType(raw[1]))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorRoot {
    pub device: Vec<TopologicalUSBDescriptorDevice>,
    pub others: Vec<USBDescriptor>,
}

#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorDevice {
    pub data: Device,
    pub child: Vec<TopologicalUSBDescriptorConfiguration>,
}

#[derive(Clone, Debug)]
pub struct TopologicalUSBDescriptorConfiguration {
    pub data: Configuration,
    pub child: Vec<TopologicalUSBDescriptorFunction>,
}

#[derive(Clone, Debug)]
pub enum TopologicalUSBDescriptorFunction {
    InterfaceAssociation((InterfaceAssociation, Vec<TopologicalUSBDescriptorFunction>)), //maybe we would have multi layer compose device in future? for now just treat it as a trick!
    Interface((Vec<(Interface, Vec<USBDescriptor>, Vec<Endpoint>)>)),
}

impl<O> RawDescriptorParser<O>
where
    O: PlatformAbstractions,
{
    pub fn new(raw: DMA<[u8], O::DMA>) -> Self {
        let len = raw.len();

        Self {
            raw,
            current: 0,
            len,
        }
    }

    pub fn parse_root_descriptors(
        &mut self,
        only_parse_device: bool,
    ) -> TopologicalUSBDescriptorRoot {
        trace!("parse root desc!");
        let mut devices = Vec::new();
        let mut other = Vec::new();
        while self.current < self.len && self.raw[self.current] > 0 {
            if let Some(desc_type) = self.peek_desc_type() {
                match desc_type {
                    DescriptorType::Device => {
                        if only_parse_device {
                            devices.push(self.parse_single_device_descriptor().unwrap());
                        } else {
                            devices.push(self.parse_device_descriptor().unwrap());
                        }
                    }
                    _ => match self.parse_any_descriptor() {
                        Ok(t) => other.push(t),
                        Err(e) => {
                            trace!("Unrecognized USB descriptor: {:?}", e)
                        }
                    },
                }
            } else {
                break;
            }
        }

        TopologicalUSBDescriptorRoot {
            device: devices,
            others: other,
        }
    }

    fn parse_any_descriptor(&mut self) -> Result<USBDescriptor, Error> {
        trace!("parse any desc! type:{:?}", self.peek_desc_type());
        let raw = self.cut_raw_descriptor()?;
        USBDescriptor::from_slice(&raw)
    }

    pub fn parse_single_device_descriptor(
        &mut self,
    ) -> Result<TopologicalUSBDescriptorDevice, Error> {
        trace!("parse single device desc!");
        let raw = self.cut_raw_descriptor()?;
        let mut device = USBDescriptor::from_slice(&raw).and_then(|converted| {
            if let USBDescriptor::Device(dev) = converted {
                Ok(TopologicalUSBDescriptorDevice {
                    data: dev,
                    child: Vec::new(),
                })
            } else {
                Err(Error::ParseOrderError)
            }
        })?;

        Ok(device)
    }

    fn parse_device_descriptor(&mut self) -> Result<TopologicalUSBDescriptorDevice, Error> {
        trace!("parse device desc!");
        let raw = self.cut_raw_descriptor()?;
        let mut device = USBDescriptor::from_slice(&raw).and_then(|converted| {
            if let USBDescriptor::Device(dev) = converted {
                Ok(TopologicalUSBDescriptorDevice {
                    data: dev,
                    child: Vec::new(),
                })
            } else {
                Err(Error::ParseOrderError)
            }
        })?;

        for _ in 0..device.data.num_configurations {
            device.child.push(self.parse_config_descriptor()?);
        }

        Ok(device)
    }

    pub fn parse_config_descriptor(
        &mut self,
    ) -> Result<TopologicalUSBDescriptorConfiguration, Error> {
        trace!("parse config desc!");
        let raw = self.cut_raw_descriptor()?;

        let mut cfg = USBDescriptor::from_slice(&raw).and_then(|converted| {
            if let USBDescriptor::Configuration(cfg) = converted {
                Ok(TopologicalUSBDescriptorConfiguration {
                    data: cfg,
                    child: Vec::new(),
                })
            } else {
                Err(Error::ParseOrderError)
            }
        })?;

        trace!("num of interfaces:{}", cfg.data.num_interfaces());

        loop {
            match self.parse_function() {
                Ok(func) => {
                    cfg.child.push(func);
                }
                Err(Error::EndOfDescriptors) => {
                    break;
                }
                Err(other) => return Err(other),
            }
        }

        Ok(cfg)
    }

    fn parse_function(&mut self) -> Result<TopologicalUSBDescriptorFunction, Error> {
        trace!("parse function desc!");
        if let Some(desc_type) = self.peek_desc_type() {
            match desc_type {
                DescriptorType::Interface => {
                    trace!("parse single interface desc!");
                    // let collections = TopologicalUSBDescriptorFunction::Interface(vec![]);
                    let mut interfaces = Vec::new();

                    let firstone = self.peek_interface().unwrap(); //at least had one, safe

                    loop {
                        match self.peek_interface() {
                            Some(next) if next.interface_number == firstone.interface_number => {
                                trace!("peeked {:#?},compare with {:#?}", next, firstone);
                                trace!("loop for interface desc!");
                                let interface = self.parse_interface().unwrap();
                                trace!("got interface {:?}", interface);
                                let additional = self.parse_other_non_endpoint_interface_childs();
                                let endpoints = self.parse_endpoints();
                                interfaces.push((interface, additional, endpoints))
                            }
                            _ => break,
                        };
                    }

                    Ok(TopologicalUSBDescriptorFunction::Interface(interfaces))
                }
                DescriptorType::InterfaceAssociation => {
                    trace!("parse InterfaceAssociation desc!");
                    let interface_association = self.parse_interface_association().unwrap();
                    let mut interfaces = Vec::new();
                    for _ in 0..interface_association.interface_count {
                        //agreement:there is always some interfaces that match the cound behind association
                        interfaces.push(self.parse_function()?);
                    }
                    Ok(TopologicalUSBDescriptorFunction::InterfaceAssociation((
                        interface_association,
                        interfaces,
                    )))
                }
                anyother => {
                    trace!("unrecognize type!");
                    Err(Error::UnrecognizedType(anyother as u8))
                }
            }
        } else {
            Err(Error::EndOfDescriptors)
        }
    }

    fn parse_interface_association(&mut self) -> Result<InterfaceAssociation, Error> {
        match self.parse_any_descriptor()? {
            USBDescriptor::InterfaceAssociation(interface_association) => Ok(interface_association),
            _ => Err(Error::ParseOrderError),
        }
    }

    fn parse_other_non_endpoint_interface_childs(&mut self) -> Vec<USBDescriptor> {
        debug!("parse additional data for interface");
        let mut vec = Vec::new();
        while self.peek_desc_type() != Some(DescriptorType::Endpoint) {
            // match self.parse_any_descriptor().unwrap() {
            //     ignore => {
            //         warn!("mismatched descriptor:{:?}", ignore)
            //     }
            // }
            vec.push(self.parse_any_descriptor().unwrap())
        }
        vec
    }

    fn parse_endpoints(&mut self) -> Vec<Endpoint> {
        debug!("parse enedpoints");
        let mut endpoints = Vec::new();
        while self.peek_desc_type() == Some(DescriptorType::Endpoint) {
            match self.parse_any_descriptor().unwrap() {
                USBDescriptor::Endpoint(endpoint) => endpoints.push(endpoint),
                _ => {}
            }
        }
        endpoints
    }

    fn parse_interface(&mut self) -> Result<Interface, Error> {
        debug!("parse interfaces");
        match self.parse_any_descriptor()? {
            USBDescriptor::Interface(int) => Ok(int),
            _ => Err(Error::ParseOrderError),
        }
    }

    fn cut_raw_descriptor(&mut self) -> Result<Vec<u8>, Error> {
        if self.current < self.len && self.raw[self.current] > 0 {
            let len: usize = self.raw[self.current].into();
            let v = self.raw[self.current..(self.current + len)].to_vec();
            self.current += len;
            Ok(v)
        } else {
            Err(Error::EndOfDescriptors)
        }
    }

    fn peek_desc_type(&mut self) -> Option<DescriptorType> {
        let peeked = DescriptorType::from_u8(self.raw[self.current + 1] as u8);
        trace!("peeked type:{:?}", peeked);
        peeked
    }

    fn peek_interface(&mut self) -> Option<Interface> {
        trace!("peek at {},value:{}", self.current, self.raw[self.current]);
        if self.peek_desc_type() == Some(DescriptorType::Interface) {
            let len = self.raw[self.current] as usize;
            let from = self.current;
            let to = from + len - 1;
            let raw = (&self.raw[from..to]) as *const [u8];
            return unsafe { ptr::read_volatile(raw.cast()) };
        }
        None
    }
}

#[derive(Copy, Clone, FromPrimitive)]
pub enum PortSpeed {
    FullSpeed = 1,
    LowSpeed = 2,
    HighSpeed = 3,
    SuperSpeed = 4,
    SuperSpeedPlus = 5,
}
