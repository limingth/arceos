use alloc::{boxed::Box, vec::Vec};
use xhci::context::{Device, Device64Byte};
use crate::{dma::DMA, OsDep};
use alloc::alloc::Allocator;

pub struct DeviceContextList<O>
where O: OsDep 
{
    dcbaa: DMA<[u64; 256], O::DMA>,
    context_list: Vec<DMA<Device64Byte, O::DMA>>,
}
unsafe impl<O:OsDep> Sync for DeviceContextList<O>{}

impl <O> DeviceContextList<O>
where O: OsDep
{
    pub fn new(max_slots: u8, os: O)->Self{
        let a = os.dma_alloc();

        let mut dcbaa = DMA::new([0u64; 256], 64, a);
        let mut context_list = Vec::with_capacity(max_slots as _);
        for i in 0..max_slots as usize{
            let a = os.dma_alloc();
            let context = DMA::new(Device::new_64byte(), 64, a);
            dcbaa[i] = context.addr() as u64;
            context_list.push(context);
        }

        Self { dcbaa, context_list }
    }

    pub fn dcbaap(&self)->usize{
        self.dcbaa.as_ptr() as _
    }
}