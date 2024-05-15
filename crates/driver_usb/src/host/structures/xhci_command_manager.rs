use core::{f32::consts::E, usize};

use aarch64_cpu::asm::barrier::{self, SY};
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use log::debug;
use spinning_top::Spinlock;
use xhci::ring::trb::command::Allowed;

use super::{command_ring::CmdRing, registers};
use crate::dma::DMAVec;

pub(crate) struct CommandManager {
    command_ring: CmdRing,
    command_complete: bool,
    current_trb: VirtAddr,
    uch_complete_code: u8,
    uch_slot_id: u8,
}

impl CommandManager {
    pub fn enable_slot(index: usize) -> bool {}

    pub fn do_command(&mut self, trb: Allowed, slot: usize) {
        let selfTrb = self.command_ring.get_enque_trb();
        if let Some(selfTrb) = selfTrb {
            selfTrb
        }
    }
}

pub(crate) static COMMAND_MANAGER: OnceCell<Spinlock<CommandManager>> = OnceCell::uninit();

pub(crate) fn command_completed(trb: VirtAddr, uch_complete_code: u8, uch_slot_id: u8) {
    let mut command_manager = COMMAND_MANAGER.try_get().unwrap().lock();
    if command_manager.command_complete || command_manager.current_trb != trb {
        return;
    }
    command_manager.current_trb = 0.into();
    command_manager.uch_complete_code = uch_complete_code;
    command_manager.uch_slot_id = uch_slot_id;

    barrier::dmb(SY);

    command_manager.command_complete = true;
}

pub(crate) fn new() {
    let cmd_manager = CommandManager {
        command_ring: CmdRing::new(),
        command_complete: true,
        current_trb: VirtAddr::from(0),
        uch_complete_code: 0,
        uch_slot_id: 0,
    };
    registers::handle(|r| {
        r.operational.crcr.update_volatile(|c| {
            c.set_command_ring_pointer(cmd_manager.command_ring.get_ring_addr().as_usize() as u64);
            if cmd_manager.command_ring.cycle_state() != 0 {
                c.set_ring_cycle_state();
            } else {
                c.clear_ring_cycle_state();
            };
        });

        COMMAND_MANAGER
            .try_init_once(move || Spinlock::new(cmd_manager))
            .expect("Failed to initialize `CommandManager`.");
    });

    debug!("initialized!");
}
