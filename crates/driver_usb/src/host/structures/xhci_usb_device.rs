use log::error;
use xhci::{
    context::{Device, Device64Byte, DeviceHandler, Slot, SlotHandler},
    extended_capabilities::debug::ContextPointer,
    ring::trb::command::EvaluateContext,
};

use super::{
    xhci_command_manager::{CommandResult, COMMAND_MANAGER},
    xhci_slot_manager::SLOT_MANAGER,
};

pub struct XHCIUSBDevice {
    device: Device64Byte,
    slotid: u8,
}

impl XHCIUSBDevice {
    pub fn new(rootport_id: u8) -> Self {
        Self {
            device: {
                let mut new_64byte = Device::new_64byte();
                let slot_mut = new_64byte.slot_mut();
                slot_mut.set_root_hub_port_number(rootport_id);

                new_64byte
            },
            slotid: todo!(),
        }
    }
    pub fn initialize(&mut self) {
        if let Some(manager) = COMMAND_MANAGER.get() {
            match manager.lock().enable_slot() {
                CommandResult::Success(code, Some(asserted_slot_id)) => {
                    SLOT_MANAGER
                        .get()
                        .unwrap()
                        .lock()
                        .assign_device(asserted_slot_id, self.device);
                    self.slotid = asserted_slot_id; //TODO assert which field to assign!
                }
                //需要让device分配在指定的内存空间中
                _ => {
                    error!("failed to enable slot!");
                }
            }
        }
    }

    pub fn get_input_context_address_device(&mut self) {}
}
