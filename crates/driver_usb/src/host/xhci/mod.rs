#[cfg(feature = "vl805")]
pub mod vl805;
use axhal::mem::phys_to_virt;
use core::num::NonZeroUsize;
use log::{debug, info};
use xhci::{
    accessor::Mapper,
    extended_capabilities::debug::EventRingDequeuePointer,
    registers::operational::{ConfigureRegister, DeviceNotificationControl},
};

use aarch64_cpu::asm::barrier;

use crate::host::structures::{
    dcbaa, extended_capabilities, registers,
    ring::{command, event},
};

use super::{
    multitask::{self, task::Task},
    port,
};

#[derive(Clone, Copy)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

pub(crate) fn init(mmio_base: usize) {
    let mut task = axtask::spawn_raw(
        || {
            debug!("little executer running!");
            multitask::executor::Executor::new().run();
        },
        "usb_executer".into(),
        axconfig::TASK_STACK_SIZE,
    );

    info!("init statics");
    unsafe {
        registers::init(mmio_base);
        extended_capabilities::init(mmio_base);
    };

    reset_xhci_controller();
    //TODO: RELEASE BIOS OWNER SHIP FOR GENERAL SITUATION
    xhci_pair_port();
    dcbaa::init();

    let mut command = command::Ring::new();
    command.init();

    registers::handle(|r| {
        r.operational.config.update_volatile(|c| {
            c.set_max_device_slots_enabled(
                r.capability
                    .hcsparams1
                    .read_volatile()
                    .number_of_device_slots(),
            );
        });
        r.operational.dnctrl.update_volatile(|c| {
            c.set(2);
        });
    });

    let mut event_ring = event::Ring::new();
    event_ring.init();

    registers::handle(|r| {
        r.operational.usbsts.update_volatile(|s| {
            s.clear_host_system_error();
            s.clear_event_interrupt();
            s.clear_port_change_detect();
            s.clear_save_restore_error();
        });
    });

    //todo add irq func
    // handle_irq(pci_irq)

    registers::handle(|r| {
        r.operational.usbcmd.update_volatile(|o| {
            o.set_controller_restore_state();
            // o.interrupter_enable();
            o.host_system_error_enable();
        });
    });

    spawn_tasks(event_ring);
}

fn xhci_pair_port() {
    // registers::handle(|r| {
    //     let count = r.port_register_set.into_iter().count();
    //     debug!("{port_nums} ports");
    //     let mut xhci_ports = vec![[0 as u8, 0xFF as u8, 0xff as u8]; count];
    //     let paramoff = r.capability.rtsoff * 4; //_rd16(base + hccparams1 + 2) * 4
    // });
    //TODO: Pair each USB 3 port with their USB 2 port,ref:https://github.com/foliagecanine/tritium-os/blob/master/kernel/arch/i386/usb/xhci.c#L423
}

fn reset_xhci_controller() {
    registers::handle(|r| {
        debug!("stop");
        r.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("wait until halt");
        while !r.operational.usbsts.read_volatile().hc_halted() {}
        debug!("halted");

        debug!("HCRST!");
        r.operational.usbcmd.update_volatile(|c| {
            c.set_host_controller_reset();
        });

        while r.operational.usbcmd.read_volatile().host_controller_reset()
            || r.operational.usbsts.read_volatile().controller_not_ready()
        {}

        debug!("Reset xHCI Controller Globally");
    });
}

fn handle_irq(pci_irq: usize) {}

fn ensure_no_error() {
    registers::handle(|r| {
        let s = r.operational.usbsts.read_volatile();

        debug!("xhci stat: {:?}", s);

        assert!(!s.hc_halted(), "HC is halted.");
        assert!(
            !s.host_system_error(),
            "An error occured on the host system."
        );
        assert!(!s.host_controller_error(), "An error occured on the xHC.");
    });
}

fn spawn_tasks(e: event::Ring) {
    port::spawn_all_connected_port_tasks();
    multitask::add(Task::new_poll(event::task(e)));
}
