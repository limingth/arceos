use crate::{addr::VirtAddr, err::*, OsDep};
use alloc::{borrow::ToOwned, format};
use axhal::irq::IrqHandler;
use core::{alloc::Allocator, num::NonZeroUsize};
use log::*;
use spinlock::SpinNoIrq;
mod registers;
use registers::Registers;
mod context;
mod ring;
use self::{context::DeviceContextList, ring::Ring};
use core::mem;

use super::{Controller, USBHostConfig};

const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;
const TAG: &str = "[XHCI]";

pub struct Xhci<O>
where
    O: OsDep,
{
    config: USBHostConfig<O>,
    regs: Registers,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    devicelist: DeviceContextList<O>,
    ring: SpinNoIrq<Ring<O>>,
}

impl<O> Controller<O> for Xhci<O>
where
    O: OsDep,
{
    fn new(config: USBHostConfig<O>) -> Result<Self>
    where
        Self: Sized,
    {
        let mmio_base = config.base_addr;
        debug!("{TAG} base addr: {:?}", mmio_base);
        let mut regs = registers::new_registers(mmio_base);

        // TODO: pcie 未配置，读不出来
        // let version = self.regs.capability.hciversion.read_volatile();
        // info!("xhci version: {:x}", version.get());
        let hcsp1 = regs.capability.hcsparams1.read_volatile();
        let max_slots = hcsp1.number_of_device_slots();
        let max_ports = hcsp1.number_of_ports();
        let max_irqs = hcsp1.number_of_interrupts();
        let page_size = regs.operational.pagesize.read_volatile().get();
        debug!(
            "{TAG} max_slots: {}, max_ports: {}, max_irqs: {}, page size: {}",
            max_slots, max_ports, max_irqs, page_size
        );

        let devicelist = DeviceContextList::new(max_slots, config.os.clone());

        // Create the command ring with 4096 / 16 (TRB size) entries, so that it uses all of the
        // DMA allocation (which is at least a 4k page).
        let entries_per_page = 4096 / mem::size_of::<ring::Trb>();
        let ring = Ring::new(config.os.clone(), entries_per_page, true)?;

        let mut s = Self {
            config,
            regs,
            max_slots,
            max_irqs,
            max_ports,
            devicelist,
            ring: SpinNoIrq::new(ring),
        };
        s.init()?;

        info!("{TAG} init success");
        Ok(s)
    }
}

impl<O> Xhci<O>
where
    O: OsDep,
{
    fn init(&mut self) -> Result {
        self.reset()?;

        self.regs.operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(self.max_slots);
        });

        self.start()?;
        Ok(())
    }

    fn reset(&mut self) -> Result {
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

        while self
            .regs
            .operational
            .usbcmd
            .read_volatile()
            .host_controller_reset()
            || self
                .regs
                .operational
                .usbsts
                .read_volatile()
                .controller_not_ready()
        {}

        info!("{TAG} XCHI reset ok");
        Ok(())
    }

    fn check_slot(&self, slot: u8) -> Result {
        if slot > self.max_slots {
            return Err(Error::Param(format!(
                "slot {} > max {}",
                slot, self.max_slots
            )));
        }
        Ok(())
    }

    fn start(&mut self) -> Result {
        debug!("{TAG} start");

        self.regs.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while self.regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} is running");
        Ok(())
    }
}
