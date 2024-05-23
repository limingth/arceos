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


use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use tock_registers::register_structs;
use tock_registers::interfaces::Writeable;

register_structs! {
    ScratchpadBufferEntry{
        (0x000 => addr_low: ReadWrite<u32>),
        (0x004 => addr_high: ReadWrite<u32>),
        (0x008 => @END),
    }
}




// pub struct ScratchpadBufferArray<O>
// where O: OsDep 
// {
//     pub entries: DMA<[ScratchpadBufferEntry], O>,
//     pub pages: Vec<DMA<[u8; O::PAGE_SIZE], O>>,
// }
// impl <O> ScratchpadBufferArray <O>
// where O: OsDep 
// {
//     pub fn new(ac64: bool, entries: u16) -> Result<Self> {
        
//         let mut entries = unsafe { Xhci::alloc_dma_zeroed_unsized_raw(ac64, entries as usize)? };

//         let pages = entries.iter_mut().map(|entry: &mut ScratchpadBufferEntry| -> Result<_, syscall::Error> {
//             let dma = unsafe { Dma::<[u8; PAGE_SIZE]>::zeroed()?.assume_init() };
//             assert_eq!(dma.physical() % PAGE_SIZE, 0);
//             entry.set_addr(dma.physical() as u64);
//             Ok(dma)
//         }).collect::<Result<Vec<_>, _>>()?;

//         Ok(Self {
//             entries,
//             pages,
//         })
//     }
//     pub fn register(&self) -> usize {
//         self.entries.physical()
//     }
// }


