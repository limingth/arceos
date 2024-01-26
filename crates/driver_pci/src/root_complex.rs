use crate::err::*;
use crate::types::*;
use crate::Access;
use crate::PciAddress;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use log::*;
use tock_registers::interfaces::{ReadWriteable, Readable};
use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};
const MAX_BUS: usize = 256;
const MAX_DEVICES: usize = 32;
const MAX_FUNCTIONS: usize = 8;

/// The root complex of a PCI bus.
#[derive(Debug, Clone)]
pub struct PciRootComplex<A: Access> {
    mmio_base: usize,
    _marker: PhantomData<A>,
}

impl<A: Access> PciRootComplex<A> {
    pub fn new(mmio_base: usize) -> Self {
        A::setup(mmio_base);
        Self {
            mmio_base,
            _marker: PhantomData::default(),
        }
    }
}

impl<A: Access> PciRootComplex<A> {
    /// Enumerates PCI devices on the given bus.
    pub fn enumerate_bus(&self) -> BusDeviceIterator<A> {
        // Safe because the BusDeviceIterator only reads read-only fields.
        let root = Self {
            mmio_base: self.mmio_base,
            _marker: PhantomData::default(),
        };
        BusDeviceIterator {
            root,
            next: PciAddress {
                bus: 0,
                device: 0,
                function: 0,
            },
            stack: Vec::new(),
        }
    }
}
/// An iterator which enumerates PCI devices and functions on a given bus.
pub struct BusDeviceIterator<A: Access> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: PciRootComplex<A>,
    next: PciAddress,
    stack: Vec<PciAddress>,
}

impl<A: Access> BusDeviceIterator<A> {}

impl<A: Access> Iterator for BusDeviceIterator<A> {
    type Item = (PciAddress, DeviceFunctionInfo);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.function >= MAX_FUNCTIONS {
                self.next.function = 0;
                self.next.device += 1;
            }

            if self.next.device >= MAX_DEVICES {
                if let Some(parent) = self.stack.pop() {
                    let sub = self.next.bus;
                    self.next.bus = parent.bus;
                    self.next.device = parent.device + 1;
                    self.next.function = 0;
                    let cfg_addr = A::map_conf(self.root.mmio_base, parent.clone()).unwrap();
                    let bridge = ConifgPciPciBridge::new(cfg_addr);
                    debug!("Bridge {} set subordinate: {:X}", parent, sub);
                    bridge.set_subordinate_bus_number(sub as _);
                } else {
                    return None;
                }
            }

            let current = self.next.clone();

            let cfg_addr = match A::map_conf(self.root.mmio_base, current.clone()) {
                Some(c) => c,
                None => {
                    if current.function == 0 {
                        self.next.device += 1;
                    } else {
                        self.next.function += 1;
                    }
                    continue;
                }
            };

            // debug!("begin: {} @ 0x{:X}", current, cfg_addr);
            let header = PciHeader::new(cfg_addr);
            let (vid, did) = header.vendor_id_and_device_id();
            // debug!("vid {:X}, did {:X}", vid, did);

            if vid == 0xffff {
                if current.function == 0 {
                    self.next.device += 1;
                } else {
                    self.next.function += 1;
                }
                continue;
            }
            let multi = header.has_multiple_functions();

            let header_type = header.header_type();
            let (dv, bc, sc, interface) = header.revision_and_class();
            let mut info = DeviceFunctionInfo::default();
            info.vendor_id = vid;
            info.device_id = did;
            info.revision = dv;
            info.class = bc;
            info.subclass = sc;
            info.header_type = header_type;
            info.prog_if = interface;
            let out = (current.clone(), info);
            match header_type {
                HeaderType::PciPciBridge => {
                    let bridge = ConifgPciPciBridge::new(cfg_addr);
                    self.stack.push(current.clone());
                    self.next.bus += 1;
                    self.next.device = 0;
                    self.next.function = 0;
                    bridge.set_secondary_bus_number(self.next.bus as _);
                    bridge.set_subordinate_bus_number(0xff);
                    A::probe_bridge(self.root.mmio_base, &bridge);
                }
                HeaderType::Endpoint => {
                    if current.function == 0 && !multi {
                        self.next.device += 1;
                    } else {
                        self.next.function += 1;
                    }
                }
                _ => {
                    if current.function == 0 && !multi {
                        self.next.device += 1;
                    } else {
                        self.next.function += 1;
                    }
                }

            }

            return Some(out);
        }

        None
    }

    
    
}

/// Information about a PCI device function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceFunctionInfo {
    /// The PCI vendor ID.
    pub vendor_id: u16,
    /// The PCI device ID.
    pub device_id: u16,
    /// The PCI class.
    pub class: u8,
    /// The PCI subclass.
    pub subclass: u8,
    /// The PCI programming interface byte.
    pub prog_if: u8,
    /// The PCI revision ID.
    pub revision: u8,
    /// The type of PCI device.
    pub header_type: HeaderType,
}

impl Default for DeviceFunctionInfo {
    fn default() -> Self {
        Self {
            header_type: HeaderType::PciPciBridge,
            vendor_id: 0,
            device_id: 0,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
        }
    }
}

impl Display for DeviceFunctionInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:04x}:{:04x} (class {:02x}.{:02x}, rev {:02x}) {:?}",
            self.vendor_id,
            self.device_id,
            self.class,
            self.subclass,
            self.revision,
            self.header_type,
        )
    }
}
