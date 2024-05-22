use crate::{dma::DMA, OsDep};
pub use super::ring::{Ring, TrbData};
use crate::err::*;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use tock_registers::register_structs;
use tock_registers::interfaces::Writeable;
use xhci::ring::trb::{self, event::Allowed};

register_structs! {
    EventRingSte {
        (0x000 => addr_low: ReadWrite<u32>),
        (0x004 => addr_high: ReadWrite<u32>),
        (0x008 => size: ReadWrite<u16>),
        (0x00A => _reserved),
        (0x010 => @END),
    }
}



pub struct EventRing<O>
where O: OsDep
{
    pub ring: Ring<O>,
    pub ste: DMA<[EventRingSte], O::DMA>
}



impl <O>EventRing <O>
where O: OsDep
{
    pub fn new(os: O) -> Result<Self> {
        let a = os.dma_alloc();
        let mut ring = EventRing {
            ste: DMA::zeroed( 1, 64, a),
            ring: Ring::new(os, 256, false)?,
        };
        ring.ste[0].addr_low.set(ring.ring.register() as u32);
        ring.ste[0].addr_high.set((ring.ring.register() as u64 >> 32) as u32);
        ring.ste[0].size.set(ring.ring.trbs.len() as u16);

        Ok(ring)
    }

    pub fn next(&mut self) -> Allowed {
        let data = self.ring.next_data().0.clone();
        Allowed::try_from(data).unwrap()
    }

    pub fn erdp(&self) -> u64 {
        self.ring.register() & 0xFFFF_FFFF_FFFF_FFF0
    }
    pub fn erstba(&self) -> u64 {
        let ptr = &self.ste[0];
        ptr as *const EventRingSte as usize as u64
    }
}

