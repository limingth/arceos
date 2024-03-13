#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::{string::String, sync::Arc, vec};
use axhal::{mem::phys_to_virt, time::busy_wait};
use core::{
    borrow::{Borrow, BorrowMut},
    cmp::min,
    num::NonZeroUsize,
    time::Duration,
};
use log::{debug, info};
use page_table_entry::aarch64;
use spinlock::SpinNoIrq;
use spinning_top::Spinlock;
use xhci::{
    accessor::Mapper, registers::capability::CapabilityParameters1, ExtendedCapability, Registers,
};

use aarch64_cpu::asm::barrier;

use crate::host::{
    exchanger,
    structures::{
        dcbaa,
        extended_capabilities::{self},
        registers,
        ring::{command, event},
        scratchpad,
    },
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
    registers::handle(|r| {});
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

fn run() {
    info!("run!");
    registers::handle(|r| {
        let o = &mut r.operational;
        o.usbcmd.update_volatile(|u| {
            u.set_run_stop();
        });

        info!("out: wait until halt");
        while o.usbsts.read_volatile().hc_halted() {
            // info!("wait until halt");
            // barrier::isb(barrier::SY);
            if o.usbsts.read_volatile().host_system_error() {
                panic!("xhci stat: {:?}", o.usbsts.read_volatile());
            }
        }

        // info!("out: wait until not halt");
        // while !o.usbsts.read_volatile().hc_halted() {
        //     // info!("wait until not halt");
        //     // barrier::isb(barrier::SY);
        // }
    });
}

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
