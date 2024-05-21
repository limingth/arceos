use crate::OsDep;
use alloc::vec::Vec;
use alloc::boxed::Box;
use log::debug;
use super::registers::Registers;
use crate::err::*;

pub type Trb = [u32; 4];


pub struct Ring<O: OsDep> {
    pub link: bool,
    pub trbs: Vec<Box<Trb, O::DMA>>,
    pub i: usize,
    pub cycle: bool,
}


impl<O: OsDep> Ring<O> {
    pub fn new(os: O, len: usize, link: bool)->Result< Self>{
        let mut trbs = Vec::with_capacity(len);
        for _ in 0..len{
            let a = os.dma_alloc();
            let trb = Box::new_in([0; 4], a);
            trbs.push(trb);
        }
        Ok(Self { link, trbs, i: 0, cycle: link })
    }
}

