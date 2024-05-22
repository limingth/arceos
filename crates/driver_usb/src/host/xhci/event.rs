use crate::{dma::DMA, OsDep};

pub use super::ring::{Ring, TrbData};
use crate::err::*;

// #[derive(Default)]
// #[repr(packed)]
// pub struct EventRingSte {
//     pub address_low: u32,
//     pub address_high: u32,
//     pub size: u16,
//     _rsvd: u16,
//     _rsvd2: u32,
// }



use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use tock_registers::register_structs;
use tock_registers::interfaces::Writeable;

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

    pub fn next(&mut self) -> &mut TrbData {
        self.ring.next().0
    }
    pub fn erdp(&self) -> u64 {
        self.ring.register() & 0xFFFF_FFFF_FFFF_FFF0
    }
    pub fn erstba(&self) -> u64 {
        let ptr = &self.ste[0];
        ptr as *const EventRingSte as usize as u64
    }
}

