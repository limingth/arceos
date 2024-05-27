use alloc::collections::VecDeque;
use conquer_once::spin::Lazy;
use core::{future::Future, pin::Pin, task::Poll};
use futures_util::task::AtomicWaker;
use init::fully_operational::FullyOperational;
use log::{info, warn};
use spinning_top::Spinlock;

use super::structures::registers;

mod endpoint;
mod init;
mod spawner;

static CURRENT_RESET_PORT: Lazy<Spinlock<ResetPort>> =
    Lazy::new(|| Spinlock::new(ResetPort::new()));

struct ResetPort {
    resetting: bool,
}
impl ResetPort {
    fn new() -> Self {
        Self { resetting: false }
    }

    fn complete_reset(&mut self) {
        self.resetting = false;
    }

    fn resettable(&mut self) -> bool {
        if self.resetting {
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

fn main(port_number: u8) {
    let mut fully_operational = init_port_and_slot_exclusively(port_number);

    fully_operational.issue_nop_trb();
}

fn init_port_and_slot_exclusively(port_number: u8) -> FullyOperational {
    let reset_waiter = ResetWaiterFuture;
    reset_waiter.poll();

    let fully_operational = init::init(port_number);
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
impl ResetWaiterFuture {
    pub fn poll(&self) {
        loop {
            if CURRENT_RESET_PORT.lock().resettable() {
                return;
            }
        }
    }
}
