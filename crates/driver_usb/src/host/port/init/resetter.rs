use crate::host::structures::registers;

use super::slot_structures_initializer::SlotStructuresInitializer;
use log::debug;
use xhci::registers::PortRegisterSet;

pub(super) struct Resetter {
    port_number: u8,
}
impl Resetter {
    pub(super) fn new(port_number: u8) -> Self {
        Self { port_number }
    }

    pub(super) fn port_number(&self) -> u8 {
        self.port_number
    }

    pub(super) fn reset(self) -> SlotStructuresInitializer {
        self.start_resetting();
        self.wait_until_reset_is_completed();
        SlotStructuresInitializer::new(self)
    }

    fn start_resetting(&self) {
        self.update_port_register(|port| {
            debug!("before reset Port, status: {:?}", port.portsc);
            port.portsc.set_0_port_enabled_disabled();
            port.portsc.set_port_reset();
        });
    }

    fn wait_until_reset_is_completed(&self) {
        while !self.reset_completed() {}
        // self.update_port_register(|p| {
        //     // p.portsc.clear_port_reset_change();
        // });
        debug!(
            "reset complete, state: {:?}",
            self.read_port_register(|port| { port.portsc })
        );
    }

    fn reset_completed(&self) -> bool {
        self.read_port_register(|r| r.portsc.port_reset_change())
    }

    fn read_port_register<T, U>(&self, f: T) -> U
    where
        T: FnOnce(&PortRegisterSet) -> U,
    {
        registers::handle(|r| {
            f(&r.port_register_set
                .read_volatile_at((self.port_number - 1).into()))
        })
    }

    fn update_port_register<T>(&self, f: T)
    where
        T: FnOnce(&mut PortRegisterSet),
    {
        registers::handle(|r| {
            r.port_register_set
                .update_volatile_at((self.port_number - 1).into(), f)
        })
    }
}
