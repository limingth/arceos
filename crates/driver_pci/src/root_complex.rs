use crate::err::*;
use crate::Access;
use crate::Address;
use crate::types::ConifgPciPciBridge;
use crate::types::PciHeader;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use log::*;
pub use pci_types::PciAddress;
use crate::types::HeaderType;
use tock_registers::interfaces::ReadWriteable;
use tock_registers::interfaces::Readable;
use tock_registers::registers::ReadOnly;
use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

const MAX_DEVICES: usize = 256;
const MAX_FUNCTIONS: u8 = 8;

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
            next: Address {
                bus: 0,
                device: 0,
                function: 0,
            },
            pri: 0,
            pri_dev: 0,
            sec: 0,
            sub: 0,
            multiple_functions: false,
        }
    }
}
impl<A: Access> pci_types::ConfigRegionAccess for PciRootComplex<A> {
    fn function_exists(&self, address: pci_types::PciAddress) -> bool {
        true
    }

    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        unsafe {
            let ptr = A::map_conf(self.mmio_base, address.into()) as *const u32;
            ptr.offset(offset as isize).read_volatile()
        }
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        unsafe {
            let ptr = A::map_conf(self.mmio_base, address.into()) as *mut u32;
            ptr.offset(offset as isize).write_volatile(value);
        }
    }
}

/// A PCI Configuration Access Mechanism.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Cam {
    /// The PCI memory-mapped Configuration Access Mechanism.
    ///
    /// This provides access to 256 bytes of configuration space per device function.
    MmioCam,
    /// The PCIe memory-mapped Enhanced Configuration Access Mechanism.
    ///
    /// This provides access to 4 KiB of configuration space per device function.
    Ecam,
}

impl Cam {
    /// Returns the total size in bytes of the memory-mapped region.
    pub const fn size(self) -> u32 {
        match self {
            Self::MmioCam => 0x1000000,
            Self::Ecam => 0x10000000,
        }
    }
}

/// An iterator which enumerates PCI devices and functions on a given bus.
pub struct BusDeviceIterator<A: Access> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: PciRootComplex<A>,
    next: Address,
    pri: u8,
    pri_dev: u8,
    sec: u8,
    sub: u8,
    multiple_functions: bool,
}

impl<A: Access> BusDeviceIterator<A> {
    fn step(&mut self, current: &Address) {
        if (current.function == 0) {
            if self.multiple_functions {
                self.next.function += 1;
                if self.next.function >= MAX_FUNCTIONS {
                    self.next.function = 0;
                    self.next.device += 1;
                }
            } else {
                self.next.device += 1;
            }

            if (self.next.device as usize) >= MAX_DEVICES {
                self.next.device = 0;
                self.next.bus += 1;
            }
        }
    }
}

impl<A: Access> Iterator for BusDeviceIterator<A> {
    type Item = (PciAddress, DeviceFunctionInfo);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current = self.next;
            let cfg_addr = A::map_conf(self.root.mmio_base, current);
            debug!("begin: {} @ 0x{:X}", current, cfg_addr);

            let header = PciHeader::new(cfg_addr);
            let (vid, did) = header.vendor_id_and_device_id();

            debug!("vid {:x}, did {:x}", vid, did);
            self.multiple_functions = header.has_multiple_functions();
            debug!("has multiple functions: {}", self.multiple_functions);

            if vid == 0xffff || vid == 0 {
                self.step(&current);
                continue;
            }

            let header_type = header.header_type();
            // let (dv, bc, sc, interface) = header.revision_and_class(access);
            let mut info = DeviceFunctionInfo::default();
            info.vendor_id = vid;
            info.device_id = did;
            // info.revision = dv;
            // info.class = bc;
            // info.subclass = sc;
            info.header_type = header_type;
            // info.prog_if = interface;
            let out_addr: PciAddress = current.into();
            let out = (out_addr, info);
            match header_type {
                HeaderType::PciPciBridge=>{
                    let bridge = ConifgPciPciBridge::new(cfg_addr);
                    debug!("bridge ");
                    self.next.bus+=1;
                    self.next.device = 0;
                    self.next.function = 0;
                    bridge.set_secondary_bus_number(self.next.bus);
                    if !self.multiple_functions{
                        bridge.set_subordinate_bus_number(self.next.bus);
                    }
                }
                _ => {
                    if (current.function == 0) {
                        if self.multiple_functions {
                            self.next.function += 1;
                            if self.next.function >= MAX_FUNCTIONS {
                                self.next.function = 0;
                                self.next.device += 1;
                            }
                        } else {
                            self.next.device += 1;
                        }

                        if (self.next.device as usize) >= MAX_DEVICES {
                            self.next.device = 0;
                            self.next.bus += 1;
                        }
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

