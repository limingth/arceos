mod structures;
pub mod xhci;
use crate::{addr::VirtAddr, dma::DMAAllocator, err::*};
use axhal::mem::PhysAddr;
use core::alloc::Allocator;
use driver_common::BaseDriverOps;
pub use xhci::Xhci;

/// The information of the graphics device.
#[derive(Debug, Clone, Copy)]
pub struct USBHostInfo {}

/// Operations that require a graphics device driver to implement.
pub trait USBHostDriverOps: BaseDriverOps {}

#[derive(Clone)]
pub struct USBHostConfig
{
    pub(crate) base_addr: VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
}

impl USBHostConfig {
    pub fn new(mmio_base_addr: usize, irq_num: u32, irq_priority: u32)->Self{
        let base_addr = VirtAddr::from(mmio_base_addr);
        Self { base_addr, irq_num, irq_priority }
    }
}



pub trait USBHostImp {
    fn new(config: USBHostConfig) -> Result<Self> where Self: Sized;
}

pub struct USBHost< U>
where
    U: USBHostImp,
{
    pub(crate) config: USBHostConfig,
    pub(crate) usb: U,
}

impl <U>USBHost<U>
where
    U: USBHostImp
{
    pub fn new(config: USBHostConfig) -> Result<Self> {
        let usb = U::new(config.clone())?;
        Ok(Self { config, usb })
    }
}
