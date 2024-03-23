use core::f32::consts::E;

use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use spinning_top::Spinlock;

use super::{command_ring::CmdRing, registers};
use crate::dma::DMAVec;

pub(crate) struct CommandManager {
    command_ring: CmdRing,
    command_complete: bool,
    current_trb: VirtAddr,
    uch_complete_code: u8,
    uch_slot_id: u8,
}

pub(crate) static COMMAND_MANAGER: OnceCell<Spinlock<CommandManager>> = OnceCell::uninit();

pub(crate) fn new() {
    registers::handle(|r| {
        let cmd_manager = CommandManager {
            command_ring: CmdRing::new(),
            command_complete: true,
            current_trb: VirtAddr::from(0),
            uch_complete_code: 0,
            uch_slot_id: 0,
        };

        r.operational.crcr.update_volatile(|c| {
            c.set_command_ring_pointer(cmd_manager.command_ring.get_ring_addr().as_usize() as u64);
            if cmd_manager.command_ring.cycle_state() {
                c.set_ring_cycle_state()
            } else {
                c.clear_ring_cycle_state()
            }
        });

        COMMAND_MANAGER
            .try_init_once(move || Spinlock::new(cmd_manager))
            .expect("Failed to initialize `CommandManager`.");
    })
}
