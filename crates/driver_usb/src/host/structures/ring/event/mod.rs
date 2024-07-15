use crate::host::{exchanger::receiver, page_box::PageBox, port, structures::registers};

use super::CycleBit;
use alloc::vec::Vec;
use axhal::mem::VirtAddr;
use bit_field::BitField;
use conquer_once::spin::OnceCell;
use core::{convert::TryInto, pin::Pin};
use log::{debug, info, warn};
use page_table::PageSize;
use segment_table::SegmentTable;
use spinning_top::Spinlock;
use xhci::ring::trb::{
    self,
    event::{self, CompletionCode},
};

mod segment_table;

static EVENT_RING: OnceCell<Spinlock<Ring>> = OnceCell::uninit();

pub fn init() {
    let ring = Spinlock::new(Ring::new());
    ring.lock().init();

    EVENT_RING
        .try_init_once(|| ring)
        .expect("`EVENT_RING` is initialized more than once.");
}

//TODO use this
pub(crate) fn poll() {
    debug!("This is the Event ring task.");

    loop {
        let trb = EVENT_RING
            .get()
            .expect("The event ring is not initialized")
            .try_lock()
            .expect("Failed to lock the event ring.")
            .next();
        {
            if let event::Allowed::CommandCompletion(x) = trb {
                debug!("complete ! {:?}", x);
                assert_eq!(x.completion_code(), Ok(CompletionCode::Success));

                receiver::receive(trb);
                return;
            } else if let event::Allowed::TransferEvent(x) = trb {
                debug!("transfer! {:?}", x);
                assert_eq!(x.completion_code(), Ok(CompletionCode::Success));

                receiver::receive(trb);
            } else if let event::Allowed::PortStatusChange(p) = trb {
                debug!("status change! {:?}", p);
                let _ = port::try_spawn(p.port_id());
                debug!("evt spawnned!");
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
const MAX_NUM_OF_TRB_IN_QUEUE: u16 = PageSize::Size4K as u16 / trb::BYTES as u16;

pub(crate) struct Ring {
    segment_table: SegmentTable,
    raw: Raw,
}
impl Ring {
    pub(crate) fn new() -> Self {
        let max_num_of_erst = registers::handle(|r| {
            r.capability
                .hcsparams2
                .read_volatile()
                .event_ring_segment_table_max()
        });

        Self {
            segment_table: SegmentTable::new(max_num_of_erst.into()),
            raw: Raw::new(),
        }
    }

    pub(crate) fn init(&mut self) {
        self.init_dequeue_ptr();
        self.init_tbl();
    }

    fn init_dequeue_ptr(&mut self) {
        self.raw.update_deq_p_with_xhci()
    }

    fn virt_addr_to_segment_table(&self) -> VirtAddr {
        self.segment_table.virt_addr()
    }

    fn init_tbl(&mut self) {
        SegTblInitializer::new(self).init();
    }

    fn try_dequeue(&mut self) -> Option<event::Allowed> {
        self.raw.try_dequeue()
    }

    fn ring_addrs(&self) -> Vec<VirtAddr> {
        self.raw.head_addrs()
    }

    fn iter_tbl_entries_mut(&mut self) -> impl Iterator<Item = &mut segment_table::Entry> {
        self.segment_table.iter_mut()
    }
}
impl Ring {
    pub fn next(&mut self) -> event::Allowed {
        loop {
            if let Some(allowed) = self.try_dequeue() {
                return allowed;
            }
        }
    }
}

struct Raw {
    rings: Vec<PageBox<[[u32; 4]]>>,
    c: CycleBit,
    deq_p_seg: usize,
    deq_p_trb: usize,
}
impl Raw {
    fn new() -> Self {
        let rings = Self::new_rings();
        Self {
            rings,
            c: CycleBit::new(true),
            deq_p_seg: 0,
            deq_p_trb: 0,
        }
    }

    fn new_rings() -> Vec<PageBox<[[u32; 4]]>> {
        let mut v = Vec::new();
        for _ in 0..Self::max_num_of_erst() {
            v.push(PageBox::new_slice([0; 4], MAX_NUM_OF_TRB_IN_QUEUE.into()));
        }

        v
    }

    fn max_num_of_erst() -> u16 {
        registers::handle(|r| {
            r.capability
                .hcsparams2
                .read_volatile()
                .event_ring_segment_table_max()
        })
    }

    fn try_dequeue(&mut self) -> Option<event::Allowed> {
        if self.empty() {
            None
        } else {
            self.dequeue()
        }
    }

    fn empty(&self) -> bool {
        self.c_bit_of_next_trb() != self.c
    }

    fn c_bit_of_next_trb(&self) -> CycleBit {
        let t = self.rings[self.deq_p_seg][self.deq_p_trb];
        CycleBit::new(t[3].get_bit(0))
    }

    fn dequeue(&mut self) -> Option<event::Allowed> {
        let t = self.get_next_trb().ok();
        self.increment();
        t
    }

    fn get_next_trb(&self) -> Result<event::Allowed, [u32; 4]> {
        let r = self.rings[self.deq_p_seg][self.deq_p_trb];
        let t = r.try_into();
        if t.is_err() {
            warn!("Unrecognized ID: {}", r[3].get_bits(10..=15));
        }
        t
    }

    fn increment(&mut self) {
        self.deq_p_trb += 1;
        if self.deq_p_trb >= MAX_NUM_OF_TRB_IN_QUEUE.into() {
            self.deq_p_trb = 0;
            self.deq_p_seg += 1;

            if self.deq_p_seg >= self.num_of_erst() {
                self.deq_p_seg = 0;
                self.c.toggle();
            }
        }
    }

    fn num_of_erst(&self) -> usize {
        self.rings.len()
    }

    fn update_deq_p_with_xhci(&self) {
        registers::handle(|r| {
            let _ = &self;

            r.interrupter_register_set
                .interrupter_mut(0)
                .erdp
                .update_volatile(|r| {
                    r.set_event_ring_dequeue_pointer(self.next_trb_addr().as_usize() as u64)
                });
        });
    }

    fn next_trb_addr(&self) -> VirtAddr {
        self.rings[self.deq_p_seg].virt_addr() + trb::BYTES * self.deq_p_trb
    }

    fn head_addrs(&self) -> Vec<VirtAddr> {
        self.rings.iter().map(PageBox::virt_addr).collect()
    }
}

struct SegTblInitializer<'a> {
    ring: &'a mut Ring,
}
impl<'a> SegTblInitializer<'a> {
    fn new(ring: &'a mut Ring) -> Self {
        Self { ring }
    }

    fn init(&mut self) {
        self.write_addrs();
        self.register_tbl_sz();
        self.enable_event_ring();
    }

    fn write_addrs(&mut self) {
        let addrs = self.ring.ring_addrs();
        for (entry, addr) in self.ring.iter_tbl_entries_mut().zip(addrs) {
            entry.set(addr, MAX_NUM_OF_TRB_IN_QUEUE);
        }
    }

    fn register_tbl_sz(&mut self) {
        registers::handle(|r| {
            let l = self.tbl_len();

            r.interrupter_register_set
                .interrupter_mut(0)
                .erstsz
                .update_volatile(|r| r.set(l.try_into().unwrap()));
        })
    }

    fn enable_event_ring(&mut self) {
        registers::handle(|r| {
            let a = self.tbl_addr();

            r.interrupter_register_set
                .interrupter_mut(0)
                .erstba
                .update_volatile(|r| {
                    r.set(a.as_usize() as u64);
                })
        });
    }

    fn tbl_addr(&self) -> VirtAddr {
        self.ring.virt_addr_to_segment_table()
    }

    fn tbl_len(&self) -> usize {
        self.ring.segment_table.len()
    }
}
