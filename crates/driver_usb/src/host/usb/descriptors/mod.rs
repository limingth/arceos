use core::{any::Any, ptr};

use alloc::{collections::BTreeMap, vec::Vec};
use axalloc::GlobalNoCacheAllocator;
use desc_configuration::Configuration;
use desc_device::Device;
use desc_endpoint::Endpoint;
use desc_interface::Interface;
use log::debug;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use crate::{dma::DMA, OsDep};

pub mod desc_configuration;
pub mod desc_device;
pub mod desc_endpoint;
pub mod desc_interface;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Descriptor {
    Device(Device),
    Configuration(Configuration),
    Str,
    Interface(Interface),
    Endpoint(Endpoint),
    Hid,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum Type {
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

#[derive(Debug)]
pub(crate) enum Error {
    UnrecognizedType(u8),
}

pub(crate) struct RawDescriptorParser<O: OsDep> {
    raw: DMA<[u8], O::DMA>,
    current: usize,
    len: usize,
}

pub(crate) struct DescriptionTypeIndexPairForControlTransfer {
    ty: Type,
    i: u8,
}

impl Descriptor {
    pub(crate) fn from_slice(raw: &[u8]) -> Result<Self, Error> {
        assert_eq!(raw.len(), raw[0].into());
        match FromPrimitive::from_u8(raw[1]) {
            Some(t) => {
                let raw: *const [u8] = raw;
                match t {
                    // SAFETY: This operation is safe because the length of `raw` is equivalent to the
                    // one of the descriptor.
                    Type::Device => Ok(Self::Device(unsafe { ptr::read(raw.cast()) })),
                    Type::Configuration => {
                        Ok(Self::Configuration(unsafe { ptr::read(raw.cast()) }))
                    }
                    Type::String => Ok(Self::Str),
                    Type::Interface => Ok(Self::Interface(unsafe { ptr::read(raw.cast()) })),
                    Type::Endpoint => Ok(Self::Endpoint(unsafe { ptr::read(raw.cast()) })),
                    Type::Hid => Ok(Self::Hid),
                    other => unimplemented!("please implement descriptor type:{:?}", other),
                }
            }
            None => Err(Error::UnrecognizedType(raw[1])),
        }
    }
}

impl<O> RawDescriptorParser<O>
where
    O: OsDep,
{
    pub fn new(raw: DMA<[u8], O::DMA>) -> Self {
        let len = raw.len();

        Self {
            raw,
            current: 0,
            len,
        }
    }

    pub fn parse(&mut self, empty_vec: &mut Vec<Descriptor>) {
        while self.current < self.len && self.raw[self.current] > 0 {
            match self.parse_first_descriptor() {
                Ok(t) => empty_vec.push(t),
                Err(e) => debug!("Unrecognized USB descriptor: {:?}", e),
            }
        }
    }

    fn parse_first_descriptor(&mut self) -> Result<Descriptor, Error> {
        let raw = self.cut_raw_descriptor();
        Descriptor::from_slice(&raw)
    }

    fn cut_raw_descriptor(&mut self) -> Vec<u8> {
        let len: usize = self.raw[self.current].into();
        let v = self.raw[self.current..(self.current + len)].to_vec();
        self.current += len;
        v
    }
}

impl Type {
    pub(crate) fn value_for_transfer_control_index(
        self,
        index: u8,
    ) -> DescriptionTypeIndexPairForControlTransfer {
        DescriptionTypeIndexPairForControlTransfer { ty: self, i: index }
    }
}

impl DescriptionTypeIndexPairForControlTransfer {
    pub(crate) fn bits(self) -> u16 {
        (self.ty as u16) << 8 | u16::from(self.i)
    }
}
