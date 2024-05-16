#[cfg(feature = "phytium-xhci")]
pub mod vl805;

use alloc::borrow::ToOwned;
use axhal::{irq::IrqHandler, mem::phys_to_virt};
use core::{alloc::Allocator, num::NonZeroUsize};
use log::*;
use xhci::{
    extended_capabilities::debug::EventRingDequeuePointer,
    registers::operational::{ConfigureRegister, DeviceNotificationControl},
    ExtendedCapability,
    Registers, 
    accessor::Mapper,
};
use aarch64_cpu::asm::barrier;

use crate::{
    addr::VirtAddr,
    dma::DMAAllocator,
    err::*,
    host::structures::{
        extended_capabilities,
        roothub::{self, Roothub},
        scratchpad, xhci_command_manager, xhci_event_manager, xhci_slot_manager,
    },
};

use super::{structures::registers, USBHostConfig, USBHostImp};

const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;

#[derive(Clone, Copy)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        // let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        // return NonZeroUsize::new_unchecked(virt.as_usize());
        return NonZeroUsize::new_unchecked(phys_base);
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

pub(crate) fn init(mmio_base: usize) {
    unsafe {
        registers::init(mmio_base);
        extended_capabilities::init(mmio_base);
    };

    debug!("resetting xhci controller");
    reset_xhci_controller();

    xhci_slot_manager::new();
    xhci_event_manager::new();
    xhci_command_manager::new();
    scratchpad::new();
    scratchpad::assign_scratchpad_into_dcbaa();
    roothub::new();

    axhal::irq::register_handler(ARM_IRQ_PCIE_HOST_INTA, interrupt_handler);
    registers::handle(|r| {
        r.operational.usbcmd.update_volatile(|r| {
            r.interrupter_enable();
            r.set_run_stop();
        })
    });

    debug!(
        "init completed!, coltroller state:{:?}",
        registers::handle(|r| r.operational.usbsts.read_volatile())
    );
}

fn interrupt_handler() {
    debug!("interrupt!");
    registers::handle(|r| {
        r.operational.usbsts.update_volatile(|sts| {
            sts.clear_event_interrupt();
        });

        r.interrupter_register_set
            .interrupter_mut(0)
            .iman
            .update_volatile(|iman| {
                iman.clear_interrupt_pending();
            });

        if r.operational.usbsts.read_volatile().hc_halted() {
            error!("HC halted");
            return;
        }

        for tries in 0..XHCI_CONFIG_MAX_EVENTS_PER_INTR {
            if xhci_event_manager::handle_event().is_ok() {}
        }
    })
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

        // debug!("get bios ownership");
        // for c in extended_capabilities::iter()
        //     .unwrap()
        //     .filter_map(Result::ok)
        // {
        //     if let ExtendedCapability::UsbLegacySupport(mut u) = c {
        //         let l = &mut u.usblegsup;
        //         l.update_volatile(|s| {
        //             s.set_hc_os_owned_semaphore();
        //         });

        //         while l.read_volatile().hc_bios_owned_semaphore()
        //             || !l.read_volatile().hc_os_owned_semaphore()
        //         {}
        //     }
        // }

        debug!("Reset xHCI Controller Globally");
    });
}

const TAG: &str = "[XHCI]";

pub struct Xhci {
    config: USBHostConfig,
    regs: xhci::Registers<MemoryMapper>,
}

impl USBHostImp for Xhci {
    fn new(config: USBHostConfig) -> Result<Self>
    where
        Self: Sized,
    {
        let mmio_base = config.base_addr.as_usize();
        debug!("{TAG} base addr: {:X}", mmio_base);
        let regs = unsafe { xhci::Registers::new(mmio_base, MemoryMapper) };
        let mut s = Self { config, regs };
        s.init()?;
        Ok(s)
    }
}

impl Xhci {
    fn init(&mut self) -> Result {
        self.reset()?;
        let version = self.regs.capability.hciversion.read_volatile();
        info!("xhci version: {:x}", version.get());
        Ok(())
    }

    fn reset(&mut self)->Result{
        debug!("{TAG} reset begin");        
        debug!("{TAG} stop");
        self.regs.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} until halt");
        while !self.regs.operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} halted");


        let mut o = &mut self.regs.operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("{TAG} wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        debug!("{TAG} ready");

        o.usbcmd.update_volatile(|f| {
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        debug!("{TAG} reset HC");

        while self.regs.operational.usbcmd.read_volatile().host_controller_reset()
            || self.regs.operational.usbsts.read_volatile().controller_not_ready()
        {}

        info!("{TAG} XCHI reset ok");
        Ok(())
    }
}
