use alloc::{borrow::ToOwned, format};
use axhal::irq::IrqHandler;
use spinlock::SpinNoIrq;
use core::{alloc::Allocator, num::NonZeroUsize};
use log::*;
use crate::{
    addr::VirtAddr,
    err::*,
};
mod registers;
use registers::Registers;
use super::{USBHostConfig, Controller};



const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;
const TAG: &str = "[XHCI]";



pub struct Xhci {
    config: USBHostConfig,
    regs: Registers,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
}

impl Controller for Xhci {
    fn new(config: USBHostConfig) -> Result<Self>
    where
        Self: Sized,
    {
        let mmio_base = config.base_addr;
        debug!("{TAG} base addr: {:?}", mmio_base);
        let regs = registers::new_registers(mmio_base);
        let mut s = Self { config, regs, max_slots:0,max_irqs:0, max_ports:0 };
        s.init()?;

        info!("{TAG} init success");
        Ok(s)
    }
}

impl Xhci {
    fn init(&mut self) -> Result {
        self.reset()?;
        // TODO: pcie 未配置，读不出来 
        // let version = self.regs.capability.hciversion.read_volatile();
        // info!("xhci version: {:x}", version.get());
        let hcsp1 = self.regs.capability.hcsparams1.read_volatile();
        let max_slots = hcsp1.number_of_device_slots();
        let max_ports = hcsp1.number_of_ports();
        let max_irqs = hcsp1.number_of_interrupts();
        let page_size = self.regs.operational.pagesize.read_volatile().get();
        debug!("{TAG} max_slots: {}, max_ports: {}, max_irqs: {}, page size: {}", max_slots, max_ports, max_irqs, page_size);
        self.max_slots = max_slots;
        self.max_ports = max_ports;
        self.max_irqs = max_irqs;

        self.regs.operational.config.update_volatile(|r|{
            r.set_max_device_slots_enabled(max_slots);
        });




        self.start()?;
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

    fn check_slot(&self, slot: u8)->Result{
        if slot > self.max_slots{
            return Err(Error::Param(format!("slot {} > max {}", slot, self.max_slots)));
        }
        Ok(())
    }


    fn start(&mut self)->Result{
        debug!("{TAG} start");

        self.regs.operational.usbcmd.update_volatile(|r|{
            r.set_run_stop();
        });

        while self.regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} is running");
        Ok(())
    }

}
