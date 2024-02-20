pub mod command_type;

use alloc::vec;
use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use axhal::mem::PhysAddr;
use command_type::CommandTrb;
use log::info;
use page_table::PageSize;
use xhci::ring::trb::{self, Link};
use xhci::{ring::trb::command, Registers};

use super::MemoryMapper;

// 定义CommandRing结构体
#[allow(clippy::cast_possible_truncation)]
const NUM_OF_TRBS: usize = PageSize::Size4K as usize / trb::BYTES;

pub struct CommandRing<'a> {
    ring: Vec<[u32; 4]>,
    enq_p: usize,
    cycle_bit: bool,
    reg: &'a mut Registers<MemoryMapper>,
}

// 实现CommandRing结构体的方法
impl CommandRing<'_> {
    // 创建一个新的命令环
    pub fn new(reg: &mut Registers<MemoryMapper>) -> Self {
        info!("new command ring");
        Self {
            ring: vec![[0; 4]; NUM_OF_TRBS],
            enq_p: 0,
            cycle_bit: true,
            reg: reg,
        }
    }

    pub(crate) fn init(&mut self) {
        let a = self.ring.as_ptr().addr();

        // Do not split this closure to avoid read-modify-write bug. Reading fields may return
        // 0, this will cause writing 0 to fields.
        self.reg.operational.crcr.update_volatile(|c| {
            c.set_command_ring_pointer(a as u64);
            c.set_ring_cycle_state();
        });
    }

    fn phys_addr(&self) -> PhysAddr {
        self.head_addr()
    }

    fn notify_command_is_sent(&self) {
        self.reg.doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_target(0);
        });
    }

    pub(crate) fn enqueue(&mut self, mut trb: command::Allowed) -> PhysAddr {
        self.set_cycle_bit(&mut trb);
        self.write_trb(trb);
        let trb_a = self.enq_addr();
        self.increment();
        self.notify_command_is_sent();
        trb_a
    }

    fn write_trb(&mut self, trb: command::Allowed) {
        // TODO: Write four 32-bit values. This way of writing is described in the spec, although
        // I cannot find which section has the description.
        self.ring[self.enq_p] = trb.into_raw();
    }

    fn increment(&mut self) {
        self.enq_p += 1;
        if !self.enq_p_within_ring() {
            self.enq_link();
            self.move_enq_p_to_the_beginning();
        }
    }

    fn enq_p_within_ring(&self) -> bool {
        self.enq_p < self.len() - 1
    }

    fn enq_link(&mut self) {
        // Don't call `enqueue`. It will return an `Err` value as there is no space for link TRB.
        let t = *Link::default().set_ring_segment_pointer(self.head_addr().as_usize() as u64);
        let mut t = command::Allowed::Link(t);
        self.set_cycle_bit(&mut t);
        self.ring[self.enq_p] = t.into_raw();
    }

    fn move_enq_p_to_the_beginning(&mut self) {
        self.enq_p = 0;
        self.cycle_bit = !self.cycle_bit;
    }

    fn enq_addr(&self) -> PhysAddr {
        self.head_addr() + trb::BYTES * self.enq_p
    }

    fn head_addr(&self) -> PhysAddr {
        PhysAddr::from(self.ring.as_ptr().addr())
    }

    fn len(&self) -> usize {
        self.ring.len()
    }

    fn set_cycle_bit(&self, trb: &mut command::Allowed) {
        if self.cycle_bit {
            trb.set_cycle_bit();
        } else {
            trb.clear_cycle_bit();
        }
    }
}
