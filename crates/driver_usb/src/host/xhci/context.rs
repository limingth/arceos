use crate::err::*;
use crate::host::usb::descriptors;
use crate::host::ControllerArc;
use crate::{dma::DMA, OsDep};
use alloc::alloc::Allocator;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::sync::Arc;
use alloc::{boxed::Box, vec::Vec};
use alloc::{format, vec};
use core::borrow::BorrowMut;
use core::num;
use log::debug;
use xhci::context::Input64Byte;
pub use xhci::context::{Device, Device64Byte, DeviceHandler};
const NUM_EPS: usize = 32;

pub struct DeviceContextList<O>
where
    O: OsDep + Clone,
{
    pub dcbaa: DMA<[u64; 256], O::DMA>,
    pub device_out_context_list: Vec<DMA<Device64Byte, O::DMA>>,
    pub device_input_context_list: Vec<DMA<Input64Byte, O::DMA>>,
    pub transfer_rings: Vec<Vec<Ring<O>>>,
    // pub attached_set: BTreeMap<usize, xhci_device::DeviceAttached<O>, O::DMA>, //大概需要加锁？
    os: O,
}

impl<O> DeviceContextList<O>
where
    O: OsDep,
{
    pub fn new(max_slots: u8, os: O) -> Self {
        let a = os.dma_alloc();

        let mut dcbaa = DMA::new([0u64; 256], 4096, a);
        let mut out_context_list = Vec::with_capacity(max_slots as _);
        let mut in_context_list = Vec::with_capacity(max_slots as _);
        for i in 0..max_slots as usize {
            let out_context = DMA::new(Device::new_64byte(), 4096, os.dma_alloc()).fill_zero();
            dcbaa[i] = out_context.addr() as u64;
            out_context_list.push(out_context);
            in_context_list
                .push(DMA::new(Input64Byte::new_64byte(), 4096, os.dma_alloc()).fill_zero());
        }
        let mut transfer_rings = Vec::with_capacity(max_slots as _);
        for _ in 0..transfer_rings.capacity() {
            transfer_rings.push(Vec::new());
        }

        Self {
            dcbaa,
            device_out_context_list: out_context_list,
            device_input_context_list: in_context_list,
            transfer_rings,
            os,
        }
    }

    pub fn dcbaap(&self) -> usize {
        self.dcbaa.as_ptr() as _
    }

    pub fn new_slot(
        &mut self,
        slot: usize,
        hub: usize,
        port: usize,
        num_ep: usize, // cannot lesser than 0, and consider about alignment, use usize
        os: O,
        controller: ControllerArc<O>,
    ) -> Result<DeviceAttached<O>> {
        if slot > self.device_out_context_list.len() {
            return Err(Error::Param(format!(
                "slot {} > max {}",
                slot,
                self.device_out_context_list.len()
            )));
        }
        let trs = (0..num_ep)
            .into_iter()
            .map(|_| Ring::new(self.os.clone(), 32, true).unwrap())
            .collect();
        debug!("new rings!");

        self.transfer_rings[slot] = trs;

        Ok(DeviceAttached::new(slot, hub, port, os, controller))
    }
}

use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};

use super::ring::Ring;
use super::xhci_device::DeviceAttached;
use super::{xhci_device, Error, Xhci};

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

unsafe impl<O: OsDep> Sync for ScratchpadBufferArray<O> {}

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
