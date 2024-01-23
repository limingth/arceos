//! Structures and functions for PCI bus operations.
//!
//! Currently, it just re-exports structures from the crate [virtio-drivers][1]
//! and its module [`virtio_drivers::transport::pci::bus`][2].
//!
//! [1]: https://docs.rs/virtio-drivers/latest/virtio_drivers/
//! [2]: https://docs.rs/virtio-drivers/latest/virtio_drivers/transport/pci/bus/index.html

#![no_std]

#[cfg(feature = "bcm2711")]
mod bcm2711;
extern crate alloc;
pub mod types;
pub mod err;
pub mod root_complex;
pub use root_complex::*;
pub use virtio_drivers::transport::pci::bus::{BarInfo, Cam, HeaderType, MemoryBarType, PciError};
pub use virtio_drivers::transport::pci::bus::{
    CapabilityInfo, Command, DeviceFunction, DeviceFunctionInfo, PciRoot, Status,
};
use pci_types::PciAddress;



#[derive(Clone)]
pub struct Address {
    bus: usize,
    device: usize,
    function: usize,
}
impl core::fmt::Display for Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02x}:{:02x}.{}", self.bus, self.device, self.function)
    }
}
impl Into<PciAddress> for Address {
    fn into(self) -> PciAddress {
        PciAddress::new(0, self.bus as _, self.device as _, self.function as _)
    }
}
impl From<PciAddress> for Address {
    fn from(value: PciAddress) -> Self {
        Self { bus: value.bus() as _ , device: value.device() as _, function: value.function() as _ }
    }
}


#[cfg(feature="bcm2711")]
pub type RootComplex = PciRootComplex<bcm2711::BCM2711>;


pub fn new_root_complex(mmio_base: usize) ->RootComplex {
    PciRootComplex::new(mmio_base)
}


pub trait Access {
    fn setup(mmio_base: usize);
    fn probe_root_complex(mmio_base: usize);
    fn map_conf(mmio_base: usize, addr: Address)->Option<usize>;
}

/// Used to allocate MMIO regions for PCI BARs.
pub struct PciRangeAllocator {
    _start: u64,
    end: u64,
    current: u64,
}

impl PciRangeAllocator {
    /// Creates a new allocator from a memory range.
    pub const fn new(base: u64, size: u64) -> Self {
        Self {
            _start: base,
            end: base + size,
            current: base,
        }
    }

    /// Allocates a memory region with the given size.
    ///
    /// The `size` should be a power of 2, and the returned value is also a
    /// multiple of `size`.
    pub fn alloc(&mut self, size: u64) -> Option<u64> {
        if !size.is_power_of_two() {
            return None;
        }
        let ret = align_up(self.current, size);
        if ret + size > self.end {
            return None;
        }

        self.current = ret + size;
        Some(ret)
    }
}

const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}
