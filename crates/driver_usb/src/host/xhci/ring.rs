use core::mem;
use core::ptr::slice_from_raw_parts;

use super::registers::Registers;
use crate::dma::DMA;
use crate::err::*;
use crate::OsDep;
use alloc::boxed::Box;
use alloc::slice;
use alloc::vec::Vec;
use log::debug;
pub use xhci::ring::trb;
use xhci::ring::trb::command::Allowed;
use xhci::ring::trb::Link;
const TRB_LEN: usize = 4;
pub type TrbData = [u32; TRB_LEN];

pub struct Ring<O: OsDep> {
    link: bool,
    pub trbs: DMA<[TrbData], O::DMA>,
    pub i: usize,
    pub cycle: bool,
}

impl<O: OsDep> Ring<O> {
    pub fn new(os: O, len: usize, link: bool) -> Result<Self> {
        let a = os.dma_alloc();
        let mut trbs = DMA::new_vec([0; TRB_LEN], len, 64, a);
        Ok(Self {
            trbs,
            i: 0,
            cycle: link,
            link,
        })
    }

    fn get_trb(&self) -> &TrbData {
        unsafe { &self.trbs[self.i] }
    }

    pub fn register(&self) -> u64 {
        self.get_trb().as_ptr() as usize as u64
    }

    pub fn enque_trb(&mut self, mut trb: TrbData) {
        debug!("enqueue trb into {}", self.i);
        self.trbs[self.i] = trb;
        let next_index = self.next_index();
        debug!("enqueued,next index: {next_index}")
    }

    pub fn enque_trbs(&mut self, trb: Vec<TrbData>) {
        for ele in trb {
            self.enque_trb(ele)
        }
    }

    fn next_index(&mut self) -> usize {
        debug!("next index");
        let mut i;
        loop {
            i = self.i;
            self.i += 1;
            if self.i >= self.trbs.len() {
                self.i = 0;
                debug!("reset index!");

                if self.link {
                    debug!("link!");
                    let address = self.trbs[self.i].as_ptr() as usize;
                    let mut link = Link::new();
                    link.set_ring_segment_pointer(address as u64)
                        .set_toggle_cycle();

                    if self.cycle {
                        link.set_cycle_bit();
                    } else {
                        link.clear_cycle_bit();
                    }
                    let trb = Allowed::Link(link);
                    let link_trb = trb.into_raw();
                    let mut this_trb = &mut self.trbs[self.i];
                    this_trb.copy_from_slice(&link_trb);

                    self.cycle = !self.cycle;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        i
    }

    pub fn next_data(&mut self) -> (&mut TrbData, bool) {
        let i = self.next_index();
        (&mut self.trbs[i], self.cycle)
    }

    pub fn peek_next_data(&mut self) -> (&TrbData, bool) {
        //may encounter non equal data
        let mut cycle_flag = self.cycle;
        let i = {
            let mut i;
            i = self.i;
            if i >= self.trbs.len() {
                i = 0;
                cycle_flag = !cycle_flag;
            }
            i
        };
        (&self.trbs[i], cycle_flag)
    }
}
