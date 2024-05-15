use core::{char::REPLACEMENT_CHARACTER, marker::PhantomData, mem};

use alloc::vec::Vec;
use axhal::mem::VirtAddr;
use futures_util::stream::All;
use page_box::PageBox;
use xhci::ring::trb::{self, event::Allowed, Link};

use crate::host::structures::XHCI_LINK_TRB_CONTROL_TC;

use super::{
    registers, XHCI_CONFIG_EVENT_RING_SIZE, XHCI_TRB_CONTROL_C, XHCI_TRB_CONTROL_TRB_TYPE_SHIFT,
};

pub struct CmdRing {
    ring: PageBox<[[u32; 4]]>,
    enque_index: usize, // 入队索引
    deque_index: usize, // 出队索引
    cycle_state: u32,   // 循环状态
}

impl CmdRing {
    /// 初始化命令环
    /// <br> 1. 初始化命令环，分配内存并设置初始化状态
    /// <br> 2. 配置Link TRB，设置环大小和循环状态
    pub fn new() -> Self {
        super::registers::handle(|r| {
            let mut command_ring = CmdRing {
                ring: PageBox::new_slice([0 as u32; 4], XHCI_CONFIG_EVENT_RING_SIZE), //TODO 此处写死256，后续可更改
                enque_index: 0,
                deque_index: 0,
                cycle_state: XHCI_TRB_CONTROL_C as u32,
            };

            let get_trb_count = command_ring.get_trb_count();
            let ringsize = command_ring.ring.virt_addr().as_usize() as u64;
            let mut link_trb = &mut command_ring.ring[get_trb_count - 1];

            unsafe { *(link_trb.as_ptr() as *mut u64) = ringsize };
            link_trb[2] = 0;
            link_trb[3] = (((xhci::ring::trb::Type::Link as usize)
                << XHCI_TRB_CONTROL_TRB_TYPE_SHIFT)
                | XHCI_LINK_TRB_CONTROL_TC) as u32;
            command_ring
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

    pub fn get_enque_trb(&mut self) -> Option<&mut [u32; 4]> {
        assert!(self.enque_index < self.get_trb_count());
        let xhci_trb = &mut self.ring[self.enque_index];
        if (xhci_trb[3] & XHCI_TRB_CONTROL_C as u32) == self.cycle_state {
            return None;
        }

        Some(xhci_trb)
        // Allowed::try_from(xhci_trb).ok().as_mut()
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

    pub fn cycle_state(&self) -> u32 {
        self.cycle_state
    }
}
