use super::{
    super::structures::ring::command,
    receiver::{self, ReceiveFuture},
};
use crate::{Futurelock, FuturelockGuard};
use alloc::sync::Arc;
use axhal::mem::VirtAddr;
use command_trb::{AddressDevice, ConfigureEndpoint, EnableSlot, EvaluateContext};
use conquer_once::spin::OnceCell;
use event::CompletionCode;
use futures_util::task::AtomicWaker;
use log::debug;
use spinning_top::Spinlock;

use xhci::ring::trb::{command as command_trb, event};

static SENDER: OnceCell<Spinlock<Sender>> = OnceCell::uninit();

pub(crate) fn init() {
    let ring = Arc::new(Spinlock::new(command::Ring::new()));

    ring.lock().init();

    SENDER
        .try_init_once(|| Spinlock::new(Sender::new(ring)))
        .expect("`Sender` is initialized more than once.")
}

pub(crate) fn enable_device_slot() -> u8 {
    lock().enable_device_slot()
}

pub(crate) fn address_device(input_cx: VirtAddr, slot: u8) {
    lock().address_device(input_cx, slot);
}

pub(crate) fn configure_endpoint(cx: VirtAddr, slot: u8) {
    lock().configure_endpoint(cx, slot);
}

pub(crate) fn evaluate_context(cx: VirtAddr, slot: u8) {
    lock().evaluate_context(cx, slot);
}

fn lock() -> spinning_top::lock_api::MutexGuard<'static, spinning_top::RawSpinlock, Sender> {
    let s = SENDER.try_get().expect("`SENDER` is not initialized.");
    s.lock()
}

struct Sender {
    channel: Channel,
}
impl Sender {
    fn new(ring: Arc<Spinlock<command::Ring>>) -> Self {
        Self {
            channel: Channel::new(ring),
        }
    }

    fn enable_device_slot(&mut self) -> u8 {
        let t = EnableSlot::default();
        let completion = self.send_and_receive(t.into());
        panic_on_error("Enable Device Slot", completion);
        if let event::Allowed::CommandCompletion(c) = completion {
            c.slot_id()
        } else {
            unreachable!()
        }
    }

    fn address_device(&mut self, input_context_addr: VirtAddr, slot_id: u8) {
        let t = *AddressDevice::default()
            .set_input_context_pointer(input_context_addr.as_usize() as u64)
            .set_slot_id(slot_id);
        let c = self.send_and_receive(t.into());
        panic_on_error("Address Device", c);
    }

    fn configure_endpoint(&mut self, context_addr: VirtAddr, slot_id: u8) {
        let t = *ConfigureEndpoint::default()
            .set_input_context_pointer(context_addr.as_usize() as u64)
            .set_slot_id(slot_id);
        let c = self.send_and_receive(t.into());
        panic_on_error("Configure Endpoint", c);
    }

    fn evaluate_context(&mut self, cx: VirtAddr, slot: u8) {
        let t = *EvaluateContext::default()
            .set_input_context_pointer(cx.as_usize() as u64)
            .set_slot_id(slot);
        let c = self.send_and_receive(t.into());
        panic_on_error("Evaluate Context", c);
    }

    fn send_and_receive(&mut self, t: command_trb::Allowed) -> event::Allowed {
        self.channel.send_and_receive(t)
    }
}

struct Channel {
    ring: Arc<Spinlock<command::Ring>>,
    waker: Arc<Spinlock<AtomicWaker>>,
}
impl Channel {
    fn new(ring: Arc<Spinlock<command::Ring>>) -> Self {
        Self {
            ring,
            waker: Arc::new(Spinlock::new(AtomicWaker::new())),
        }
    }

    fn send_and_receive(&mut self, t: command_trb::Allowed) -> event::Allowed {
        debug!("send and receive: {:?}", t);
        let a = self.ring.lock().enqueue(t);
        self.register_with_receiver(a);
        self.get_trb(a)
    }

    fn register_with_receiver(&mut self, trb_a: VirtAddr) {
        receiver::add_entry(trb_a, self.waker.clone()).expect("Sender is already registered.");
    }

    fn get_trb(&mut self, trb_a: VirtAddr) -> event::Allowed {
        ReceiveFuture::new(trb_a).poll()
    }
}

fn panic_on_error(n: &str, c: event::Allowed) {
    if let event::Allowed::CommandCompletion(c) = c {
        if c.completion_code() != Ok(CompletionCode::Success) {
            panic!("{} command failed: {:?}", n, c.completion_code());
        }
    } else {
        unreachable!("The Command Completion TRB is the only TRB to receive in response to the Command TRBs.")
    }
}
