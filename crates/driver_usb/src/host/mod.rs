pub mod xhci;
use crate::{addr::VirtAddr, dma::DMAAllocator, err::*};

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

    // fn init_dev_entry(&self, slot_id : i32)->Result{

    // }
}
