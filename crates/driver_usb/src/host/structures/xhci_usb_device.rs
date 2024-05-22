use core::{borrow::BorrowMut, time::Duration};

use aarch64_cpu::asm::barrier::{self, SY};
use alloc::{borrow::ToOwned, boxed::Box, sync::Arc};
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axtask::sleep;
use log::{debug, error};
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{Device, Device64Byte, DeviceHandler, EndpointType, Slot, SlotHandler},
    extended_capabilities::debug::ContextPointer,
    ring::trb::{
        command::{self, ConfigureEndpoint, EvaluateContext},
        event::{CompletionCode, TransferEvent},
        transfer::{self, Allowed, DataStage, Direction, SetupStage, StatusStage, TransferType},
    },
};

use crate::host::structures::{transfer_ring::TransferRing, xhci_event_manager};

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
        debug!("new device! port:{}", port_id);
        if let Some(manager) = COMMAND_MANAGER.get() {
            match manager.lock().enable_slot() {
                CommandResult::Success(succedd_trb) => {
                    debug!("enable slot success!");
                    Ok({
                        let xhciusbdevice = Self {
                            context: Context::default(),
                            transfer_ring: Box::new_in(
                                TransferRing::new(),
                                global_no_cache_allocator(),
                            ),
                            port_id,
                            slot_id: succedd_trb.slot_id(),
                        };

                        xhciusbdevice
                    })
                }
                //需要让device分配在指定的内存空间中
                err => Err({
                    error!("failed to enable slot!\n {:?}", err);
                }),
            }
        } else {
            Err({ error!("command manager not initialized! it should not happen!") })
        }
    }

    pub fn initialize(&mut self) {
        self.config_endpoint_0_assign_dev();
        self.address_device();
        sleep(Duration::from_millis(2));
        let get_descriptor = self.get_descriptor();
        debug!("get desc: {:?}", get_descriptor)
    }

    fn config_endpoint_0_assign_dev(&mut self) {
        debug!("begin config endpoint 0 and assign dev!");
        let input_control = self.context.input.control_mut();
        input_control.set_add_context_flag(0);
        input_control.set_add_context_flag(1);

        let slot = self.context.input.device_mut().slot_mut();
        slot.set_context_entries(1);
        slot.set_root_hub_port_number(self.port_id);

        let mut s = {
            let psi = registers::handle(|r| {
                r.port_register_set
                    .read_volatile_at((self.port_id).into())
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

        debug!("config ep0");
        let ep_0 = self.context.input.device_mut().endpoint_mut(1);
        let endpoint_state = ep_0.endpoint_state();
        debug!("endpoint 0 state: {:?}", endpoint_state);
        ep_0.set_endpoint_type(EndpointType::Control);
        ep_0.set_max_packet_size(s);
        ep_0.set_tr_dequeue_pointer(self.transfer_ring.get_ring_addr().as_usize() as u64);
        ep_0.set_dequeue_cycle_state();
        ep_0.set_error_count(3);

        debug!("assigning device into dcbaa");
        match &(*self.context.output) {
            super::context::Device::Byte64(device) => {
                SLOT_MANAGER.get().unwrap().lock().assign_device(
                    self.slot_id,
                    (&**device as *const Device64Byte).addr().into(),
                );
            }
            //ugly,should reform code as soon as possible
            _ => {}
        }

        //confitional compile needed
        barrier::dmb(SY);
    }

    fn address_device(&mut self) {
        debug!("addressing device");
        // let ring_addr = self.context.input.virt_addr();
        let ring_addr = self.transfer_ring.get_ring_addr();
        match COMMAND_MANAGER
            .get()
            .unwrap()
            .lock()
            .address_device(ring_addr, self.slot_id)
        {
            CommandResult::Success(trb) => {
                debug!("addressed device at slot id {}", self.slot_id);
                debug!("command result {:?}", trb);
            }
            err => error!("error while address device at slot id {}", self.slot_id),
        }
    }

    fn enqueue_trb_to_transfer(&mut self, trb: transfer::Allowed) -> Result<[u32; 4], ()> {
        self.transfer_ring.enqueue(trb);
        barrier::dmb(SY);

        debug!("doorbell ing");
        registers::handle(|r| {
            r.doorbell
                .update_volatile_at(self.slot_id as usize, |doorbell| {
                    doorbell.set_doorbell_target(1u8); //assume 1
                })
        });

        sleep(Duration::from_micros(10));

        while let handle_event = xhci_event_manager::handle_event() {
            if handle_event.is_ok() {
                debug!("interrupt handler complete! result = {:?}", handle_event);
                return handle_event;
            }
        }
        Err(())
    }

    fn get_descriptor(&mut self) -> descriptor::Device {
        debug!("get descriptor!");

        let buffer = PageBox::from(descriptor::Device::default());
        let mut has_data_stage = false;

        Ok(Allowed::SetupStage({
            let mut setup_stage = SetupStage::default(); //TODO check transfer ring
            setup_stage
                .set_transfer_type(TransferType::In)
                .clear_interrupt_on_completion()
                .set_request_type(0x80)
                .set_request(6)
                .set_value(0x0100)
                .set_length(8);
            debug!("setup stage!");
            setup_stage
        }))
        .and_then(|trb| self.enqueue_trb_to_transfer(trb))
        .map(|arg0: [u32; 4]| TransferEvent::try_from(arg0).unwrap())
        .and_then(|trb| {
            debug!(
                "optional data stage! transfer len: {}",
                trb.trb_transfer_length()
            );
            if trb.trb_transfer_length() > 0 {
                has_data_stage = true;
                self.enqueue_trb_to_transfer(transfer::Allowed::DataStage(
                    *DataStage::default()
                        .set_direction(Direction::In)
                        .clear_interrupt_on_completion()
                        // .set_trb_transfer_length(trb.trb_transfer_length())
                        .set_trb_transfer_length(8) //device to controller, so use lowest speed to ensure compability
                        .set_data_buffer_pointer(buffer.virt_addr().as_usize() as u64),
                ))
            } else {
                Ok(trb.into_raw())
            }
        })
        .map(|arg0: [u32; 4]| TransferEvent::try_from(arg0).unwrap())
        .and_then(|_| {
            debug!("status stage for check state!");
            self.enqueue_trb_to_transfer(transfer::Allowed::StatusStage(
                *StatusStage::default().set_interrupt_on_completion(),
            ))
        })
        .is_ok();

        *buffer
    }
}
