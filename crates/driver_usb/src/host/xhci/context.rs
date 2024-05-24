use core::borrow::BorrowMut;
use core::num;
use alloc::collections::BTreeMap;
use alloc::format;
use crate::{dma::DMA, OsDep};
use alloc::alloc::Allocator;
use alloc::{boxed::Box, vec::Vec};
pub use xhci::context::{Device, Device64Byte, DeviceHandler};
use crate::err::*;
const NUM_EPS: usize = 32;



pub struct DeviceAttached<O>
where O:OsDep
{
    hub: i32,
    port: i32,
    num_endp: i32,
    address: i32,
    transfer_rings: Vec<Ring<O>>,
}

pub struct DeviceContextList<O>
where
    O: OsDep,
{
    pub dcbaa: DMA<[u64; 256], O::DMA>,
    pub context_list: Vec<DMA<Device64Byte, O::DMA>>,
    pub attached_set: BTreeMap<usize, DeviceAttached<O>>,
    os : O,
}

impl<O> DeviceContextList<O>
where
    O: OsDep,
{
    pub fn new(max_slots: u8, os: O) -> Self {
        let a = os.dma_alloc();

        let mut dcbaa = DMA::new([0u64; 256], 64, a);
        let mut context_list = Vec::with_capacity(max_slots as _);
        for i in 0..max_slots as usize {
            let a = os.dma_alloc();
            let context = DMA::new(Device::new_64byte(), 64, a);
            
            dcbaa[i] = context.addr() as u64;
            context_list.push(context);
        }

        Self {
            dcbaa,
            context_list,
            attached_set: BTreeMap::new(),
            os
        }
    }

    pub fn dcbaap(&self) -> usize {
        self.dcbaa.as_ptr() as _
    }


    pub fn new_slot(&mut self, slot: usize, hub: i32, port: i32, num_ep: i32)-> Result<&mut DeviceAttached<O>>{
        if slot > self.context_list.len(){
            return  Err(Error::Param(format!("slot {} > max {}", slot, self.context_list.len())));
        }
        let trs = (0..num_ep).into_iter().map(|_| Ring::new(self.os.clone(), 16, true).unwrap());
        self.attached_set.insert(slot, DeviceAttached { hub, port, num_endp: 0, address: slot as i32, transfer_rings: trs.collect() });
                

        Ok(self.attached_set.get_mut(&slot).unwrap())
    }
}

use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};

use super::ring::Ring;
use super::Error;

register_structs! {
    ScratchpadBufferEntry{
        (0x000 => value_low: ReadWrite<u32>),
        (0x004 => value_high: ReadWrite<u32>),
        (0x008 => @END),
    }
}

impl ScratchpadBufferEntry {
    pub fn set_addr(&mut self, addr: u64) {
        self.value_low.set(addr as u32);
        self.value_high.set((addr >> 32) as u32);
    }
}

pub struct ScratchpadBufferArray<O>
where
    O: OsDep,
{
    pub entries: DMA<[ScratchpadBufferEntry], O::DMA>,
    pub pages: Vec<DMA<[u8], O::DMA>>,
}

unsafe impl      <O: OsDep> Sync for ScratchpadBufferArray<O>{
    
}

impl<O> ScratchpadBufferArray<O>
where
    O: OsDep,
{
    pub fn new(entries: u32, os: O) -> Self {
        let page_size = O::PAGE_SIZE;
        let align = 64;

        let mut entries: DMA<[ScratchpadBufferEntry], O::DMA> =
            DMA::zeroed(entries as usize, align, os.dma_alloc());

        let pages = entries
            .iter_mut()
            .map(|entry| {
                let dma = DMA::zeroed(page_size, align, os.dma_alloc());

                assert_eq!(dma.addr() % page_size, 0);
                entry.set_addr(dma.addr() as u64);
                dma
            })
            .collect();

        Self { entries, pages }
    }
    pub fn register(&self) -> usize {
        self.entries.addr()
    }
}
