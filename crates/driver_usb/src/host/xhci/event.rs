pub use super::ring::{Ring, TrbData};
use crate::err::*;
use crate::{dma::DMA, OsDep};
use log::debug;
use tock_registers::interfaces::Writeable;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
use xhci::extended_capabilities::hci_extended_power_management::Data;
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
where
    O: OsDep,
{
    pub ring: Ring<O>,
    pub ste: DMA<[EventRingSte], O::DMA>,
}

impl<O> EventRing<O>
where
    O: OsDep,
{
    pub fn new(os: O) -> Result<Self> {
        let a = os.dma_alloc();
        let mut ring = EventRing {
            ste: DMA::zeroed(1, 64, a),
            ring: Ring::new(os, 30, false)?,
        };
        ring.ste[0].addr_low.set(ring.ring.register() as u32);
        ring.ste[0]
            .addr_high
            .set((ring.ring.register() as u64 >> 32) as u32);
        ring.ste[0].size.set(ring.ring.trbs.len() as u16);

        Ok(ring)
    }

    pub fn next(&mut self) -> Option<Allowed> {
        let (data, flag) = self.ring.current_data();
        let mut allowed = Allowed::try_from(data.clone()).ok()?;

        // debug!("event: next: {:#?}", allowed);

        if flag == allowed.cycle_bit() {
            return None;
        }
        if allowed.cycle_bit() {
            allowed.clear_cycle_bit();
        } else {
            allowed.set_cycle_bit();
        }
        self.ring
            .trbs
            .get_mut(self.ring.i)
            .unwrap()
            .copy_from_slice(&allowed.into_raw());
        self.ring.next_data();
        Some(allowed)
    }

    pub fn erdp(&self) -> u64 {
        self.ring.register() & 0xFFFF_FFFF_FFFF_FFF0
    }
    pub fn erstba(&self) -> u64 {
        let ptr = &self.ste[0];
        ptr as *const EventRingSte as usize as u64
    }
}
