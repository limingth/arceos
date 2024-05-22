use crate::{addr::VirtAddr, err::*, OsDep};
use alloc::{borrow::ToOwned, format};
use axhal::irq::IrqHandler;
use core::{alloc::Allocator, num::NonZeroUsize};
use log::*;
use spinlock::SpinNoIrq;
use xhci::ring::trb::command::{Allowed, Noop};
mod registers;
use registers::Registers;
mod context;
mod event;
pub(crate) mod ring;
use self::{context::DeviceContextList, event::EventRing, ring::Ring};
use super::{Controller, USBHostConfig};
use alloc::sync::Arc;
use core::mem;
const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;
const TAG: &str = "[XHCI]";

pub struct Xhci<O>
where
    O: OsDep,
{
    config: USBHostConfig<O>,
    regs: SpinNoIrq<Registers>,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    dev_ctx: DeviceContextList<O>,
    ring: SpinNoIrq<Ring<O>>,
    primary_event_ring: SpinNoIrq<EventRing<O>>,
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

        let dev_ctx = DeviceContextList::new(max_slots, config.os.clone());

        // Create the command ring with 4096 / 16 (TRB size) entries, so that it uses all of the
        // DMA allocation (which is at least a 4k page).
        let entries_per_page = 4096 / mem::size_of::<ring::TrbData>();
        let ring = Ring::new(config.os.clone(), entries_per_page, true)?;
        let event = EventRing::new(config.os.clone())?;


        let mut s = Self {
            config,
            regs: SpinNoIrq::new(regs),
            max_slots,
            max_irqs,
            max_ports,
            dev_ctx,
            ring: SpinNoIrq::new(ring),
            primary_event_ring: SpinNoIrq::new(event),
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
        self.start()?;
        self.test_cmd()?;
        Ok(())
    }

    fn reset(&mut self) -> Result {
        debug!("{TAG} reset begin");
        debug!("{TAG} stop");

        let mut regs = self.regs.lock();

        regs.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} until halt");
        while !regs.operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} halted");

        let mut o = &mut regs.operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("{TAG} wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        debug!("{TAG} ready");

        o.usbcmd.update_volatile(|f| {
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        debug!("{TAG} reset HC");

        while regs
            .operational
            .usbcmd
            .read_volatile()
            .host_controller_reset()
            || regs
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
        let mut regs = self.regs.lock();

        let dcbaap = self.dev_ctx.dcbaap();
        debug!("Writing DCBAAP: {:X}", dcbaap);
        regs.operational.dcbaap.update_volatile(|r| {
            r.set(dcbaap as u64);
        });

        let crcr = self.ring.get_mut().register();
        debug!("Writing CRCR: {:X}", crcr);
        regs.operational.crcr.update_volatile(|r| {
            r.set_command_ring_pointer(crcr);
        });

        debug!("{TAG} Setting enabled slots to {}.", self.max_slots);
        regs.operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(self.max_slots);
        });

        debug!("{TAG} disable interrupts");

        regs.operational.usbcmd.update_volatile(|r| {
            r.clear_interrupter_enable();
        });

        let mut ir0 = regs.interrupter_register_set.interrupter_mut(0);
        {
            debug!("{TAG} Writing ERSTZ");
            ir0.erstsz.update_volatile(|r| r.set(1));

            let erdp = self.primary_event_ring.get_mut().erdp();
            debug!("Writing ERDP: {:X}", erdp);

            ir0.erdp.update_volatile(|r| {
                r.set_event_ring_dequeue_pointer(erdp);
            });

            let erstba = self.primary_event_ring.get_mut().erstba();
            debug!("Writing ERSTBA: {:X}", erstba);

            ir0.erstba.update_volatile(|r| {
                r.set(erstba);
            });

            ir0.imod.update_volatile(|im| {
                im.set_interrupt_moderation_interval(0);
                im.set_interrupt_moderation_counter(0);
            });

            debug!("Enabling Primary Interrupter.");
            ir0.iman.update_volatile(|im| {
                im.set_interrupt_enable();
            });
        }

        debug!("{TAG} start run");
        regs.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} is running");

        Ok(())
    }

    fn post_cmd(&self, trb: Allowed) -> Result {
        let mut cr = self.ring.lock();
        let (buff, cycle) = cr.next();

        let mut regs = self.regs.lock();

        regs.doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        Ok(())
    }

    fn test_cmd(&self) -> Result {
        debug!("{TAG} test command ring");
        for _ in 0..3 {
            self.post_cmd(Allowed::Noop(Noop::new()))?;
        }
        debug!("{TAG} command ring ok");
        Ok(())
    }
}
