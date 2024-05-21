use core::borrow::BorrowMut;

use alloc::{borrow::ToOwned, boxed::Box, sync::Arc};
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use log::{debug, error};
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{Device, Device64Byte, DeviceHandler, EndpointType, Slot, SlotHandler},
    extended_capabilities::debug::ContextPointer,
    ring::trb::{
        command::EvaluateContext,
        transfer::{self, Allowed, DataStage, Direction, SetupStage, StatusStage, TransferType},
    },
};

use crate::host::structures::transfer_ring::TransferRing;

use super::{
    context::Context,
    descriptor, registers,
    xhci_command_manager::{CommandResult, COMMAND_MANAGER},
    xhci_slot_manager::SLOT_MANAGER,
};

pub struct XHCIUSBDevice {
    context: Context,
    transfer_ring: Box<TransferRing, GlobalNoCacheAllocator>,
    slot_id: u8,
    port_id: u8,
}

impl XHCIUSBDevice {
    pub fn new(port_id: u8) -> Result<Self, ()> {
        // Self {
        //     context: {
        //         let mut new_64byte = Device::new_64byte();
        //         let slot_mut = new_64byte.slot_mut();
        //         slot_mut.set_root_hub_port_number(rootport_id);

        //         new_64byte
        //     },
        //     slotid: todo!(),
        // }

        if let Some(manager) = COMMAND_MANAGER.get() {
            match manager.lock().enable_slot() {
                CommandResult::Success(code, Some(asserted_slot_id)) => {
                    debug!("enable slot success!");
                    Ok({
                        let xhciusbdevice = Self {
                            context: Context::default(),
                            transfer_ring: Box::new_in(
                                TransferRing::new(),
                                global_no_cache_allocator(),
                            ),
                            port_id: port_id,
                            slot_id: asserted_slot_id,
                        };

                        debug!("return...");
                        xhciusbdevice
                    })
                    // SLOT_MANAGER
                    //     .get()
                    //     .unwrap()
                    //     .lock()
                    //     .assign_device(asserted_slot_id, self.device);
                    // self.slotid = asserted_slot_id; //TODO assert which field to assign!
                }
                //需要让device分配在指定的内存空间中
                _ => Err({
                    error!("failed to enable slot!");
                }),
            }
        } else {
            Err({ error!("command manager not initialized! it should not happen!") })
        }
    }
    pub fn initialize(&mut self) {
        debug!("device initializing...");

        let input_control = self.context.input.control_mut();
        input_control.set_add_context_flag(0);
        input_control.set_add_context_flag(1);
        let slot = self.context.input.device_mut().slot_mut();
        slot.set_context_entries(1);
        slot.set_root_hub_port_number(self.port_id);

        let s = {
            let psi = registers::handle(|r| {
                r.port_register_set
                    .read_volatile_at((self.port_id - 1).into())
                    .portsc
                    .port_speed()
            });

            match psi {
                1 | 3 => 64,
                2 => 8,
                4 => 512,
                _ => unimplemented!("PSI: {}", psi),
            }
        };
        let ep_0 = self.context.input.device_mut().endpoint_mut(1);
        ep_0.set_endpoint_type(EndpointType::Control);
        ep_0.set_max_packet_size(s);
        ep_0.set_tr_dequeue_pointer(self.transfer_ring.get_ring_addr().as_usize() as u64);
        ep_0.set_dequeue_cycle_state();
        ep_0.set_error_count(3);

        // let a = self.context.output.phys_addr();
        match &(*self.context.output) {
            super::context::Device::Byte64(device) => SLOT_MANAGER
                .get()
                .unwrap()
                .lock()
                .assign_device(self.port_id, **device),
            //ugly,should reform code as soon as possible
            _ => {}
        }

        // let virt_addr = self.context.input.virt_addr();
        // match COMMAND_MANAGER
        //     .get()
        //     .unwrap()
        //     .lock()
        //     .address_device(virt_addr, self.slot_id)
        // {
        //     CommandResult::Success(_, _) => debug!("addressed device at slot id {}", self.slot_id),
        //     err => error!("error while address device at slot id {}", self.slot_id),
        // }

        debug!(
            "device (port-{}:slot-{}) initialize complete!",
            self.port_id, self.slot_id
        );
    }
    pub(crate) fn get_max_packet_size_from_device_descriptor(&mut self) -> u16 {
        let b = PageBox::from(descriptor::Device::default());

        let setup = Allowed::SetupStage({
            let mut setup_stage = SetupStage::default(); //TODO check transfer ring
            setup_stage
                .set_transfer_type(TransferType::In)
                .clear_interrupt_on_completion()
                .set_request_type(0x80)
                .set_request(6)
                .set_value(0x0100)
                .set_length(8);
            setup_stage
        });

        let data = transfer::Allowed::DataStage(
            *DataStage::default()
                .set_direction(Direction::In)
                .set_trb_transfer_length(8)
                .clear_interrupt_on_completion()
                .set_data_buffer_pointer(b.virt_addr().as_usize() as u64),
        );

        let status =
            transfer::Allowed::StatusStage(*StatusStage::default().set_interrupt_on_completion());

        self.issue_trbs(&[setup.into(), data.into(), status.into()]);

        b.max_packet_size()
    }

    fn issue_trbs(&mut self, ts: &[transfer::Allowed]) {
        for ele in ts.iter() {
            let allowed = self.transfer_ring.get_enque_trb().unwrap();
            // allowed =
        }
    }
}
