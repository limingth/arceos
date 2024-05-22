use core::mem;
use core::ptr::slice_from_raw_parts;

use super::registers::Registers;
use crate::dma::{DMAVec, DMA};
use crate::err::*;
use crate::OsDep;
use alloc::boxed::Box;
use alloc::slice;
use log::debug;
pub use xhci::ring::trb;
const TRB_LEN: usize = 4;
pub type TrbData = [u32; TRB_LEN];

pub struct Ring<O: OsDep> {
    pub link: bool,
    trbs: DMA<[TrbData], O::DMA>,
    pub i: usize,
    pub cycle: bool,
}

impl<O: OsDep> Ring<O> {
    pub fn new(os: O, len: usize, link: bool) -> Result<Self> {
        let a = os.dma_alloc();
        let mut trbs = DMA::new_vec([0; TRB_LEN], len, 64, a);
        Ok(Self {
            link,
            trbs,
            i: 0,
            cycle: link,
        })
    }

    fn get_trb(&self) -> &TrbData {
        unsafe {
            &self.trbs[self.i]
        }
    }

    pub fn register(&self) -> u64 {
        self.get_trb().as_ptr() as usize as u64
    }

    fn next_index(&mut self) -> usize {
        let i = self.i;
        self.i += 1;
        i
    }
}
