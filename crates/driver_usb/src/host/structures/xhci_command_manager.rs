use core::{f32::consts::E, usize};

use aarch64_cpu::asm::barrier::{self, SY};
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use log::debug;
use spinning_top::Spinlock;
use xhci::{
    context::Slot,
    ring::trb::command::{AddressDevice, Allowed, DisableSlot, EnableSlot},
};

use super::{command_ring::CmdRing, registers, XHCI_CONFIG_MAX_SLOTS};
use crate::dma::DMAVec;

pub(crate) struct CommandManager {
    command_ring: CmdRing,
    command_complete: bool,
    current_trb: VirtAddr,
    uch_complete_code: u8,
    uch_slot_id: u8,
}

type SlotID = u8;
pub(crate) enum CommandResult {
    Success(u8, Option<SlotID>),
    NoSlotsAvailableError,
    ShortPacket,
    RingUnderrun,
    RingOverrun,
    EventRingFullError,
    MissedServiceError,
}

impl CommandManager {
    fn slot_id_in_valid_range(slotid: u8) -> bool {
        (1..=XHCI_CONFIG_MAX_SLOTS).contains(&(slotid as usize))
    }

    pub fn disable_slot(&mut self, slotid: SlotID) -> CommandResult {
        if Self::slot_id_in_valid_range(slotid) {
            let disable_slot = Allowed::DisableSlot({
                let mut disable_slot = DisableSlot::new();
                disable_slot.set_slot_id(slotid);
                disable_slot
            });
            return self.do_command(disable_slot);
        }
        CommandResult::NoSlotsAvailableError
    }

    pub fn enable_slot(&mut self) -> CommandResult {
        self.do_command(Allowed::EnableSlot(EnableSlot::new()))
    }

    pub fn address_device(&mut self, addr: VirtAddr, slot_id: u8) -> CommandResult {
        self.do_command(Allowed::AddressDevice({
            let mut address_device = AddressDevice::default();
            address_device
                .set_input_context_pointer(addr.as_usize() as u64)
                .set_slot_id(slot_id);
            address_device
        }))
    }

    pub fn do_commands(&mut self, trb: &[Allowed]) {
        trb.iter().for_each(|trb| {
            self.do_command(*trb);
        })
    }

    pub fn do_command(&mut self, trb: Allowed) -> CommandResult {
        debug!("do command {:?} !", trb);
        //todo check
        assert!(self.command_complete);
        let mut trb1 = trb.into_raw();
        trb1[3] |= self.command_ring.cycle_state(); //weird
        if let Some(poped) = self.command_ring.get_enque_trb() {
            *poped = trb1;
            self.command_complete = false;
            self.command_ring.inc_enque();

            registers::handle(|r| {
                r.doorbell.update_volatile_at(0, |d| {
                    d.set_doorbell_stream_id(0);
                    d.set_doorbell_target(0);
                });
                //TODO: suspect, view
            });
            debug!("waiting for interrupt handler complete!");
            while (!self.command_complete) {}
            debug!("interrupt handler complete!");
            if Self::slot_id_in_valid_range(self.uch_slot_id) {
                return CommandResult::Success(self.uch_complete_code, Some(self.uch_slot_id));
            } else {
                return CommandResult::NoSlotsAvailableError;
            }
        } else {
            return CommandResult::RingOverrun;
        }
    }

    pub fn command_ring_ptr(&self) -> VirtAddr {
        self.command_ring.get_ring_addr()
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
