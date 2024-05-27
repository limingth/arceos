use alloc::{collections::BTreeMap, sync::Arc};
use axhal::mem::VirtAddr;
use conquer_once::spin::Lazy;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::task::AtomicWaker;
use spinning_top::{guard::SpinlockGuard, Spinlock};

use xhci::ring::trb::event;

static RECEIVER: Lazy<Spinlock<Receiver>> = Lazy::new(|| Spinlock::new(Receiver::new()));

pub(crate) fn add_entry(trb_a: VirtAddr, waker: Arc<Spinlock<AtomicWaker>>) -> Result<(), Error> {
    lock().add_entry(trb_a, waker)
}

pub(crate) fn receive(t: event::Allowed) {
    lock().receive(t)
}

fn lock() -> SpinlockGuard<'static, Receiver> {
    RECEIVER
        .try_lock()
        .expect("Failed to acquire the lock of `RECEIVER`.")
}

struct Receiver {
    trbs: BTreeMap<VirtAddr, Option<event::Allowed>>,
    wakers: BTreeMap<VirtAddr, Arc<Spinlock<AtomicWaker>>>,
}
impl Receiver {
    fn new() -> Self {
        Self {
            trbs: BTreeMap::new(),
            wakers: BTreeMap::new(),
        }
    }

    fn add_entry(
        &mut self,
        addr_to_trb: VirtAddr,
        waker: Arc<Spinlock<AtomicWaker>>,
    ) -> Result<(), Error> {
        if self.trbs.insert(addr_to_trb, None).is_some() {
            return Err(Error::AddrAlreadyRegistered);
        }

        if self.wakers.insert(addr_to_trb, waker).is_some() {
            return Err(Error::AddrAlreadyRegistered);
        }
        Ok(())
    }

    fn receive(&mut self, trb: event::Allowed) {
        if let Err(e) = self.insert_trb(trb) {
            panic!("Failed to receive a command completion trb: {:?}", e);
        }
    }

    fn insert_trb(&mut self, trb: event::Allowed) -> Result<(), Error> {
        let addr_to_trb = Self::trb_addr(trb);
        {
            let addr_to_trb = Self::trb_addr(trb);
            *self
                .trbs
                .get_mut(&addr_to_trb)
                .ok_or(Error::NoSuchAddress)? = Some(trb);
            Ok(())
        }?;
        Ok(())
    }

    fn trb_arrives(&self, addr_to_trb: VirtAddr) -> bool {
        match self.trbs.get(&addr_to_trb) {
            Some(trb) => trb.is_some(),
            None => panic!("No such TRB with the address {:?}", addr_to_trb),
        }
    }

    fn remove_entry(&mut self, addr_to_trb: VirtAddr) -> Option<event::Allowed> {
        match self.trbs.remove(&addr_to_trb) {
            Some(trb) => trb,
            None => panic!("No such receiver with TRB address: {:?}", addr_to_trb),
        }
    }

    fn trb_addr(t: event::Allowed) -> VirtAddr {
        VirtAddr::from(match t {
            event::Allowed::TransferEvent(e) => e.trb_pointer() as usize,
            event::Allowed::CommandCompletion(c) => c.command_trb_pointer() as usize,
            _ => todo!(),
        })
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    AddrAlreadyRegistered,
    NoSuchAddress,
}

pub(crate) struct ReceiveFuture {
    addr_to_trb: VirtAddr,
}
impl ReceiveFuture {
    pub(crate) fn new(addr_to_trb: VirtAddr) -> Self {
        Self { addr_to_trb }
    }

    pub fn poll(&mut self) -> event::Allowed {
        crate::host::structures::ring::event::poll();
        let addr = self.addr_to_trb;
        let mut r = lock();

        loop {
            if r.trb_arrives(addr) {
                return r.remove_entry(addr).unwrap();
            }
        }
    }
}
