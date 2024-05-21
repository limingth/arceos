use alloc::{boxed::Box, vec::Vec};
use xhci::context::{Device, Device64Byte};
use crate::OsDep;
use alloc::alloc::Allocator;

pub struct DeviceContextList<O>
where O: OsDep 
{
    dcbaa: Vec<u64, O::DMA>,
    context_list: Vec<Box<Device64Byte, O::DMA>>,
}


impl <O> DeviceContextList<O>
where O: OsDep
{
    pub fn new(max_slots: u8, os: O)->Self{
        let a = os.dma_alloc();
        let mut dcbaa = Vec::with_capacity_in(max_slots as usize, a);
        let mut context_list = Vec::with_capacity(dcbaa.capacity());
        for _ in 0..max_slots{
            let a = os.dma_alloc();
            let context = Box::new_in(Device::new_64byte(), a);
        
            dcbaa.push(context.as_ref() as *const _ as usize as u64);
            context_list.push(context);
        }

        Self { dcbaa, context_list }
    }
}