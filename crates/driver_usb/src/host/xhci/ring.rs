use core::mem;
use core::ptr::slice_from_raw_parts;

use crate::dma::DMAVec;
use crate::OsDep;
use alloc::slice;
use alloc::boxed::Box;
use log::debug;
use super::registers::Registers;
use crate::err::*;

const TRB_LEN: usize = 4;
pub type Trb = [u32; TRB_LEN];


pub struct Ring<O: OsDep> {
    pub link: bool,
    trbs: DMAVec<u32, O::DMA>,
    pub i: usize,
    pub cycle: bool,
}


impl<O: OsDep> Ring<O> {
    pub fn new(os: O, len: usize, link: bool)->Result< Self>{
        let trbs_len = len * TRB_LEN;
        let a = os.dma_alloc();
        let mut trbs = DMAVec::new(trbs_len, 64,a);
        Ok(Self { link, trbs, i: 0, cycle: link })
    }
    
    fn get_trb(&self)->&Trb{
        unsafe{
           let ptr = self.trbs.as_ptr().offset((self.i * TRB_LEN) as isize);
           &*(ptr as *const Trb)
        }
    }


    pub fn register(&self) -> u64 {
        self.get_trb().as_ptr() as usize as u64
    }
}

