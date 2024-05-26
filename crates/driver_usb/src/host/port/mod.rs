use super::structures::registers;
use crate::multitask::{self, task::Task};
use alloc::collections::VecDeque;
use conquer_once::spin::Lazy;
use core::{future::Future, pin::Pin, task::Poll};
use futures_util::task::AtomicWaker;
use init::fully_operational::FullyOperational;
use log::{info, warn};
use qemu_exit::{QEMUExit, X86};
use qemu_print::qemu_println;
use spinning_top::Spinlock;
use uefi::table::cfg::HAND_OFF_BLOCK_LIST_GUID;

mod class_driver;
mod endpoint;
mod init;
mod spawner;

static CURRENT_RESET_PORT: Lazy<Spinlock<ResetPort>> =
    Lazy::new(|| Spinlock::new(ResetPort::new()));

struct ResetPort {
    resetting: bool,
    wakers: VecDeque<AtomicWaker>,
}
impl ResetPort {
    fn new() -> Self {
        Self {
            resetting: false,
            wakers: VecDeque::new(),
        }
    }

    fn complete_reset(&mut self) {
        self.resetting = false;
        if let Some(w) = self.wakers.pop_front() {
            w.wake();
        }
    }

    fn resettable(&mut self, waker: AtomicWaker) -> bool {
        if self.resetting {
            self.wakers.push_back(waker);
            false
        } else {
            self.resetting = true;
            true
        }
    }
}

pub(crate) fn try_spawn(port_idx: u8) -> Result<(), spawner::PortNotConnected> {
    spawner::try_spawn(port_idx)
}

async fn main(port_number: u8) {
    qemu_println!("Port {} is connected.", port_number);

    let mut fully_operational = init_port_and_slot_exclusively(port_number).await;

    fully_operational.issue_nop_trb().await;

    qemu_println!("Port {} is fully operational.", port_number);

    let exit_handler = X86::new(0xf4, 33);

    exit_handler.exit_success();
}

async fn init_port_and_slot_exclusively(port_number: u8) -> FullyOperational {
    let reset_waiter = ResetWaiterFuture;
    reset_waiter.await;

    let fully_operational = init::init(port_number).await;
    CURRENT_RESET_PORT.lock().complete_reset();
    info!("Port {} reset completed.", port_number);
    fully_operational
}

pub(crate) fn enum_all_connected_port() {
    spawner::spawn_all_connected_ports();
}

fn max_num() -> u8 {
    registers::handle(|r| r.capability.hcsparams1.read_volatile().number_of_ports())
}

fn connected(port_number: u8) -> bool {
    registers::handle(|r| {
        r.port_register_set
            .read_volatile_at((port_number - 1).into())
            .portsc
            .current_connect_status()
    })
}

struct ResetWaiterFuture;
impl Future for ResetWaiterFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let waker = AtomicWaker::new();
        waker.register(cx.waker());
        if CURRENT_RESET_PORT.lock().resettable(waker) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
