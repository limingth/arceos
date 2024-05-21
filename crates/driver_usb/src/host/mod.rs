pub mod xhci;
pub mod device;
use core::alloc::Allocator;

use alloc::{boxed::Box, sync::Arc};
use spinlock::SpinNoIrq;

use crate::{addr::VirtAddr, err::*};

#[derive(Clone)]
pub struct USBHostConfig
{
    pub(crate) base_addr: VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) alloc: Arc<Box<dyn Allocator + Send + Sync>>,
}

impl USBHostConfig {
    pub fn new<A: Allocator + Send + Sync + 'static>(mmio_base_addr: usize, irq_num: u32, irq_priority: u32, dma_alloc:  A)->Self{
        let base_addr = VirtAddr::from(mmio_base_addr);
        let alloc: Box<dyn Allocator+Send+Sync> = Box::new(dma_alloc);
        let alloc = Arc::new(alloc);
        Self { base_addr, irq_num, irq_priority, alloc }
    }
}

pub trait Controller: Send + Sync {
    fn new(config: USBHostConfig) -> Result<Self> where Self: Sized;
}

#[derive(Clone)]
pub struct USBHost
{
    pub(crate) config: USBHostConfig,
    pub(crate) controller: Arc<SpinNoIrq<Box<dyn Controller>>>,
}

impl USBHost
{
    pub fn new<C: Controller + 'static>(config: USBHostConfig) -> Result<Self> {
        let controller : Box<dyn Controller>= Box::new(C::new(config.clone())?);
        let controller = Arc::new( SpinNoIrq::new(controller));
        Ok(Self { config, controller })
    }

    fn init_dev_entry(&self, slot_id : i32)->Result{
        


        Ok(())
    }
}



pub(crate) fn while_with_timeout<F>(f: F)->Result
where F: Fn()->bool{
    while f() {}
    Ok(())
}