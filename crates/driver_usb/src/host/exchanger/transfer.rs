use crate::host::{
    page_box::PageBox,
    structures::{descriptor, registers, ring::transfer},
};

use super::receiver::{self, receive, ReceiveFuture};
use alloc::{sync::Arc, vec::Vec};
use axhal::mem::VirtAddr;
use core::convert::TryInto;
use futures_util::task::AtomicWaker;
use log::debug;
use spinning_top::Spinlock;

use xhci::ring::trb::{
    event, transfer as transfer_trb,
    transfer::{Direction, Noop, Normal, TransferType},
};

pub(crate) struct Sender {
    channel: Channel,
}
impl Sender {
    pub(crate) fn new(doorbell_writer: DoorbellWriter) -> Self {
        Self {
            channel: Channel::new(doorbell_writer),
        }
    }

    pub(crate) fn ring_addr(&self) -> VirtAddr {
        self.channel.ring_addr()
    }

    pub(crate) fn get_max_packet_size_from_device_descriptor(&mut self) -> u16 {
        let b = PageBox::from(descriptor::Device::default());

        let setup = *transfer_trb::SetupStage::default()
            .set_transfer_type(TransferType::In)
            .clear_interrupt_on_completion()
            .set_request_type(0x80)
            .set_request(6)
            .set_value(0x0100)
            .set_length(8);

        let data = *transfer_trb::DataStage::default()
            .set_direction(Direction::In)
            .set_trb_transfer_length(8)
            .clear_interrupt_on_completion()
            .set_data_buffer_pointer(b.virt_addr().as_usize() as u64);

        let status = *transfer_trb::StatusStage::default().set_interrupt_on_completion();

        self.issue_trbs(&[setup.into(), data.into(), status.into()]);

        b.max_packet_size()
    }

    pub(crate) fn set_configure(&mut self, config_val: u8) {
        let setup = *transfer_trb::SetupStage::default()
            .set_transfer_type(TransferType::No)
            .clear_interrupt_on_completion()
            .set_request_type(0)
            .set_request(9)
            .set_value(config_val.into())
            .set_length(0);

        let status = *transfer_trb::StatusStage::default().set_interrupt_on_completion();

        self.issue_trbs(&[setup.into(), status.into()]);
    }

    pub(crate) fn set_idle(&mut self) {
        let setup = *transfer_trb::SetupStage::default()
            .set_transfer_type(TransferType::No)
            .clear_interrupt_on_completion()
            .set_request_type(0x21)
            .set_request(0x0a)
            .set_value(0)
            .set_length(0);

        let status = *transfer_trb::StatusStage::default().set_interrupt_on_completion();

        self.issue_trbs(&[setup.into(), status.into()]);
    }

    pub(crate) fn set_boot_protocol(&mut self) {
        let setup = *transfer_trb::SetupStage::default()
            .set_transfer_type(TransferType::No)
            .clear_interrupt_on_completion()
            .set_request_type(0b0010_0001)
            .set_request(0x0b)
            .set_value(0)
            .set_length(0);

        let status = *transfer_trb::StatusStage::default().set_interrupt_on_completion();

        self.issue_trbs(&[setup.into(), status.into()]);
    }

    pub(crate) fn get_configuration_descriptor(&mut self) -> PageBox<[u8]> {
        let b = PageBox::new_slice(0, 4096);

        let (setup, data, status) = Self::trbs_for_getting_descriptors(
            &b,
            DescTyIdx::new(descriptor::Ty::Configuration, 0),
        );

        self.issue_trbs(&[setup, data, status]);
        debug!("Got TRBs");
        b
    }

    pub(crate) fn issue_normal_trb<T: ?Sized>(&mut self, b: &PageBox<T>) {
        let t = *Normal::default()
            .set_data_buffer_pointer(b.virt_addr().as_usize() as u64)
            .set_trb_transfer_length(b.bytes().as_usize().try_into().unwrap())
            .set_interrupt_on_completion();
        debug!("Normal TRB: {:X?}", t);
        self.issue_trbs(&[t.into()]);
    }

    pub(crate) fn issue_nop_trb(&mut self) {
        let t = Noop::default();

        self.issue_trbs(&[t.into()]);
    }

    fn trbs_for_getting_descriptors<T: ?Sized>(
        b: &PageBox<T>,
        t: DescTyIdx,
    ) -> (
        transfer_trb::Allowed,
        transfer_trb::Allowed,
        transfer_trb::Allowed,
    ) {
        let setup = *transfer_trb::SetupStage::default()
            .set_request_type(0b1000_0000)
            .set_request(Request::GetDescriptor as u8)
            .set_value(t.bits())
            .set_length(b.bytes().as_usize().try_into().unwrap())
            .set_transfer_type(TransferType::In);

        let data = *transfer_trb::DataStage::default()
            .set_data_buffer_pointer(b.virt_addr().as_usize() as u64)
            .set_trb_transfer_length(b.bytes().as_usize().try_into().unwrap())
            .set_direction(Direction::In);

        let status = *transfer_trb::StatusStage::default().set_interrupt_on_completion();

        (setup.into(), data.into(), status.into())
    }

    fn issue_trbs(&mut self, ts: &[transfer_trb::Allowed]) -> Vec<Option<event::Allowed>> {
        self.channel.send_and_receive(ts)
    }
}

struct Channel {
    ring: transfer::Ring,
    doorbell_writer: DoorbellWriter,
    waker: Arc<Spinlock<AtomicWaker>>,
}
impl Channel {
    fn new(doorbell_writer: DoorbellWriter) -> Self {
        Self {
            ring: transfer::Ring::new(),
            doorbell_writer,
            waker: Arc::new(Spinlock::new(AtomicWaker::new())),
        }
    }

    fn ring_addr(&self) -> VirtAddr {
        self.ring.virt_addr()
    }

    fn send_and_receive(&mut self, trbs: &[transfer_trb::Allowed]) -> Vec<Option<event::Allowed>> {
        let addrs = self.ring.enqueue(trbs);
        // self.register_with_receiver(trbs, &addrs);
        self.write_to_doorbell();
        self.get_trbs(trbs, &addrs)
    }

    fn register_with_receiver(&mut self, ts: &[transfer_trb::Allowed], addrs: &[VirtAddr]) {
        for (t, addr) in ts.iter().zip(addrs) {
            self.register_trb(t, *addr);
        }
    }

    fn register_trb(&mut self, t: &transfer_trb::Allowed, a: VirtAddr) {
        if t.interrupt_on_completion() {
            receiver::add_entry(a, self.waker.clone()).expect("Sender is already registered.");
        }
    }

    fn write_to_doorbell(&mut self) {
        self.doorbell_writer.write();
    }

    fn get_trbs(
        &mut self,
        ts: &[transfer_trb::Allowed],
        addrs: &[VirtAddr],
    ) -> Vec<Option<event::Allowed>> {
        let mut v = Vec::new();
        for (t, a) in ts.iter().zip(addrs) {
            v.push(self.get_single_trb(t, *a));
        }
        v
    }

    fn get_single_trb(
        &mut self,
        t: &transfer_trb::Allowed,
        addr: VirtAddr,
    ) -> Option<event::Allowed> {
        Some(ReceiveFuture::new(addr).poll())
    }
}

pub(crate) struct DoorbellWriter {
    slot_id: u8,
    val: u32,
}
impl DoorbellWriter {
    pub(crate) fn new(slot_id: u8, val: u32) -> Self {
        Self { slot_id, val }
    }

    pub(crate) fn write(&mut self) {
        registers::handle(|r| {
            r.doorbell.update_volatile_at(self.slot_id.into(), |d| {
                d.set_doorbell_target(self.val.try_into().unwrap());
            })
        });
    }
}

pub(crate) struct DescTyIdx {
    ty: descriptor::Ty,
    i: u8,
}
impl DescTyIdx {
    pub(crate) fn new(ty: descriptor::Ty, i: u8) -> Self {
        Self { ty, i }
    }
    pub(crate) fn bits(self) -> u16 {
        (self.ty as u16) << 8 | u16::from(self.i)
    }
}

enum Request {
    GetDescriptor = 6,
}
