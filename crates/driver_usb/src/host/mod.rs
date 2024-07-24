use crate::abstractions::PlatformAbstractions;

pub mod data_structures;

#[derive(Clone)]
pub struct USBHostConfig<O>
where
    O: PlatformAbstractions,
{
    pub(crate) base_addr: O::VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) os: O,
}

impl<O> USBHostConfig<O>
where
    O: PlatformAbstractions,
{
    pub fn new(mmio_base_addr: usize, irq_num: u32, irq_priority: u32, os_dep: O) -> Self {
        let base_addr = O::VirtAddr::from(mmio_base_addr);
        Self {
            base_addr,
            irq_num,
            irq_priority,
            os: os_dep,
        }
    }
}
