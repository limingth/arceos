#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::{string::String, sync::Arc};
use axhal::mem::phys_to_virt;
use core::{
    borrow::{Borrow, BorrowMut},
    num::NonZeroUsize,
};
use log::{debug, info};
use page_table_entry::aarch64;
use spinlock::SpinNoIrq;
use spinning_top::Spinlock;
use xhci::{accessor::Mapper, ExtendedCapability, Registers};

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
    let mut event_ring = event::Ring::new();
    let command_ring = Arc::new(Spinlock::new(command::Ring::new()));

    info!("get bios perms");
    if let Some(iter) = extended_capabilities::iter() {
        for c in iter.filter_map(Result::ok) {
            if let ExtendedCapability::UsbLegacySupport(mut u) = c {
                let l = &mut u.usblegsup;
                l.update_volatile(|s| {
                    s.set_hc_os_owned_semaphore();
                });

                while l.read_volatile().hc_bios_owned_semaphore()
                    || !l.read_volatile().hc_os_owned_semaphore()
                {}
            }
        }
    }

    registers::handle(|r| {
        info!("stop");
        r.operational.usbcmd.update_volatile(|u| {
            u.clear_run_stop();
        })
    });

    registers::handle(|r| {
        info!("wait until halt");
        while !r.operational.usbsts.read_volatile().hc_halted() {}
    });

    registers::handle(|r| {
        info!("start reset");
        r.operational.usbcmd.update_volatile(|u| {
            u.set_host_controller_reset();
        });
    });

    registers::handle(|r| {
        info!("wait until reset complete");
        while r.operational.usbcmd.read_volatile().host_controller_reset() {}
    });

    registers::handle(|r| {
        info!("wait until ready");
        while r.operational.usbsts.read_volatile().controller_not_ready() {}
    });

    registers::handle(|r| {
        let n = r
            .capability
            .hcsparams1
            .read_volatile()
            .number_of_device_slots();
        info!("setting num of slots:{}", n);
        r.operational.config.update_volatile(|c| {
            c.set_max_device_slots_enabled(n);
        });
    });

    event_ring.init();

    command_ring.lock().init();

    dcbaa::init();

    scratchpad::init();

    exchanger::command::init(command_ring);

    run();
    barrier::isb(barrier::SY);
    ensure_no_error();
    spawn_tasks(event_ring);
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
