use core::{char::REPLACEMENT_CHARACTER, marker::PhantomData, mem};

use alloc::vec::Vec;
use axhal::mem::VirtAddr;
use driver_pci::types;
use futures_util::stream::All;
use page_box::PageBox;
use xhci::ring::trb::{self, event::Allowed, Link};

use super::registers;

const XHCI_CONFIG_EVENT_RING_SIZE: usize = 256;
const XHCI_TRB_CONTROL_C: usize = 1 << 0;
const XHCI_LINK_TRB_CONTROL_TC: usize = 1 << 1;

pub struct Ring {
    ring: PageBox<[[u32; 4]]>,
    enque_index: usize,
    deque_index: usize,
    cycle_state: u32,
}

impl Ring {
    pub fn new() -> Self {
        super::registers::handle(|r| {
            let event_ring = Ring {
                ring: PageBox::new_slice([0 as u32; 4], XHCI_CONFIG_EVENT_RING_SIZE), //TODO 此处写死256，后续可更改
                enque_index: 0,
                deque_index: 0,
                cycle_state: XHCI_TRB_CONTROL_C as u32,
            };

            event_ring
        })
    }

    pub fn get_trb_count(&self) -> usize {
        self.ring.len()
    }

    pub fn get_ring_addr(&self) -> VirtAddr {
        self.ring.virt_addr()
    }

    pub fn get_deque_trb(&self) -> Option<Allowed> {
        assert!(self.deque_index < self.get_trb_count());
        let xhci_trb = self.ring[self.deque_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) != self.cycle_state {
            return None;
        }

        Allowed::try_from(xhci_trb).ok()
    }

    pub fn get_enque_trb(&self) -> Option<Allowed> {
        assert!(self.enque_index < self.get_trb_count());
        let xhci_trb = self.ring[self.enque_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) == self.cycle_state {
            return None;
        }

        Allowed::try_from(xhci_trb).ok()
    }

    pub fn inc_deque(&mut self) -> Option<Allowed> {
        assert!(self.deque_index < self.get_trb_count());
        assert_eq!(
            self.ring[self.enque_index][3] & XHCI_TRB_CONTROL_C as u32,
            self.cycle_state
        );

        self.deque_index += 1;

        if self.deque_index == self.get_trb_count() {
            self.deque_index = 0;
            self.cycle_state ^= XHCI_TRB_CONTROL_C as u32;
        }

        Allowed::try_from(self.ring[self.deque_index]).ok()
    }

    pub fn inc_enque(&mut self) {
        assert!(self.enque_index < self.get_trb_count());
        assert_eq!(
            self.ring[self.enque_index][3] & XHCI_TRB_CONTROL_C as u32,
            self.cycle_state
        );

        self.enque_index += 1;

        if self.enque_index == self.get_trb_count() - 1 {
            let mut xhci_trb = self.ring[self.enque_index];

            xhci_trb[3] ^= (XHCI_TRB_CONTROL_C as u32);

            if (xhci_trb[3] & XHCI_LINK_TRB_CONTROL_TC as u32) != 0 {
                self.cycle_state ^= (XHCI_TRB_CONTROL_C as u32)
            }
        }
    }
}
