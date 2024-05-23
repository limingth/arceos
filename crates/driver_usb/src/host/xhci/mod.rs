use crate::{addr::VirtAddr, err::*, OsDep};
use alloc::{borrow::ToOwned, format, vec::Vec};
use axhal::irq::IrqHandler;
use core::{
    alloc::Allocator,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
};
use log::*;
use spinlock::SpinNoIrq;
use xhci::{
    context::Slot, registers::PortRegisterSet, ring::trb::{
        command::{Allowed, Noop},
        event::CommandCompletion,
    }
};

pub use xhci::ring::trb::transfer::Direction;
mod registers;
use registers::Registers;
mod context;

mod event;
pub(crate) mod ring;
use self::{
    context::{DeviceContextList, ScratchpadBufferArray},
    event::EventRing,
    ring::Ring,
};
use super::{Controller, USBHostConfig};
use crate::host::device::*;
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
    dev_ctx: SpinNoIrq<DeviceContextList<O>>,
    ring: SpinNoIrq<Ring<O>>,
    primary_event_ring: SpinNoIrq<EventRing<O>>,
    scratchpad_buf_arr: Option<ScratchpadBufferArray<O>>,
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
        debug!("{TAG} Base addr: {:?}", mmio_base);
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
            "{TAG} Max_slots: {}, max_ports: {}, max_irqs: {}, page size: {}",
            max_slots, max_ports, max_irqs, page_size
        );

        let dev_ctx = DeviceContextList::new(max_slots, config.os.clone());

        // Create the command ring with 4096 / 16 (TRB size) entries, so that it uses all of the
        // DMA allocation (which is at least a 4k page).
        let entries_per_page = O::PAGE_SIZE / mem::size_of::<ring::TrbData>();
        let ring = Ring::new(config.os.clone(), entries_per_page, true)?;
        let event = EventRing::new(config.os.clone())?;

        let mut s = Self {
            config,
            regs: SpinNoIrq::new(regs),
            max_slots,
            max_irqs,
            max_ports,
            dev_ctx: SpinNoIrq::new(dev_ctx),
            ring: SpinNoIrq::new(ring),
            primary_event_ring: SpinNoIrq::new(event),
            scratchpad_buf_arr: None,
        };
        s.init()?;
        info!("{TAG} Init success");
        Ok(s)
    }

    fn poll(&self) -> Result {
        todo!()
    }
}

impl<O> Xhci<O>
where
    O: OsDep,
{
    fn init(&mut self) -> Result {
        self.reset()?;
        self.init_registers()?;
        self.reset_cic();
        self.start()?;
        self.test_cmd()?;
        self.root_hub_init()?;
        // self.probe()?;
        Ok(())
    }

    fn reset(&mut self) -> Result {
        debug!("{TAG} Reset begin");
        debug!("{TAG} Stop");

        let mut regs = self.regs.lock();

        regs.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} Until halt");
        while !regs.operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} Halted");

        let mut o = &mut regs.operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("{TAG} Wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        debug!("{TAG} Ready");

        o.usbcmd.update_volatile(|f| {
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        debug!("{TAG} Reset HC");

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
        debug!("{TAG} Start run");
        regs.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} Is running");

        Ok(())
    }

    fn init_registers(&mut self) -> Result {
        let crcr = { self.ring.lock().register() };

        let buf_count = {
            let mut regs = self.regs.lock();

            let dcbaap = { self.dev_ctx.lock().dcbaap()} ;
            debug!("{TAG} Writing DCBAAP: {:X}", dcbaap);
            regs.operational.dcbaap.update_volatile(|r| {
                r.set(dcbaap as u64);
            });

            debug!("{TAG} Writing CRCR: {:X}", crcr);
            regs.operational.crcr.update_volatile(|r| {
                r.set_command_ring_pointer(crcr);
            });

            debug!("{TAG} Setting enabled slots to {}.", self.max_slots);
            regs.operational.config.update_volatile(|r| {
                r.set_max_device_slots_enabled(self.max_slots);
            });

            debug!("{TAG} Disable interrupts");

            regs.operational.usbcmd.update_volatile(|r| {
                r.clear_interrupter_enable();
            });

            let mut ir0 = regs.interrupter_register_set.interrupter_mut(0);
            {
                debug!("{TAG} Writing ERSTZ");
                ir0.erstsz.update_volatile(|r| r.set(1));

                let erdp = self.primary_event_ring.get_mut().erdp();
                debug!("{TAG} Writing ERDP: {:X}", erdp);

                ir0.erdp.update_volatile(|r| {
                    r.set_event_ring_dequeue_pointer(erdp);
                });

                let erstba = self.primary_event_ring.get_mut().erstba();
                debug!("{TAG} Writing ERSTBA: {:X}", erstba);

                ir0.erstba.update_volatile(|r| {
                    r.set(erstba);
                });

                ir0.imod.update_volatile(|im| {
                    im.set_interrupt_moderation_interval(0);
                    im.set_interrupt_moderation_counter(0);
                });

                debug!("{TAG} Enabling primary interrupter.");
                ir0.iman.update_volatile(|im| {
                    im.set_interrupt_enable();
                });
            }
            regs.capability
                .hcsparams2
                .read_volatile()
                .max_scratchpad_buffers()
        };

        self.setup_scratchpads(buf_count);

        Ok(())
    }

    fn post_cmd(&self, trb: Allowed) -> Result {
        {
            let mut cr = self.ring.lock();
            let (buff, cycle) = cr.next_data();

            let ptr = &buff[0] as *const u32 as usize;

            debug!("{TAG} Post cmd {:?} @{:X}", trb, ptr);

            let mut regs = self.regs.lock();

            regs.doorbell.update_volatile_at(0, |r| {
                r.set_doorbell_stream_id(0);
                r.set_doorbell_target(0);
            });
        }
        debug!("{TAG} Wait result");
        {
            let mut er = self.primary_event_ring.lock();
            let event = er.next();

            if let ring::trb::event::Allowed::CommandCompletion(c) = event {
                while c.completion_code().is_err() {}
                debug!("{TAG} Cmd @{:X} got result", c.command_trb_pointer());
            } else {
                warn!("{TAG} Event not match!");
            }
        }
        Ok(())
    }

    fn test_cmd(&self) -> Result {
        debug!("{TAG} Test command ring");
        for _ in 0..3 {
            self.post_cmd(Allowed::Noop(Noop::new()))?;
        }
        debug!("{TAG} Command ring ok");
        Ok(())
    }

    fn setup_scratchpads(&mut self, buf_count: u32) {
        debug!("{TAG} Scratch buf count: {}", buf_count);

        if buf_count == 0 {
            return;
        }
        let scratchpad_buf_arr = ScratchpadBufferArray::new(buf_count, self.config.os.clone());
        {
            let mut dev_ctx = self.dev_ctx.lock();
            dev_ctx.dcbaa[0] = scratchpad_buf_arr.register() as u64;

        }
        debug!(
            "{TAG} Setting up {} scratchpads, at {:#0x}",
            buf_count,
            scratchpad_buf_arr.register()
        );
        self.scratchpad_buf_arr = Some(scratchpad_buf_arr);
    }

    fn reset_cic(&self) {
        let mut regs = self.regs.lock();
        let cic = regs
            .capability
            .hccparams2
            .read_volatile()
            .configuration_information_capability();
        regs.operational.config.update_volatile(|r| {
            if cic {
                r.set_configuration_information_enable();
            } else {
                r.clear_configuration_information_enable();
            }
        });
    }

    fn root_hub_init(&self) -> Result {
        {
            let mut regs = self.regs.lock();

            let port_len = regs.port_register_set.len();

            for i in 0..port_len {
                debug!(
                    "{TAG} Port {} start reset, connected: {}",
                    i,
                    regs.port_register_set
                        .read_volatile_at(i)
                        .portsc
                        .current_connect_status()
                );
                regs.port_register_set.update_volatile_at(i, |port| {
                    port.portsc.set_port_reset();
                });
            }

            for i in 0..port_len {
                while regs
                    .port_register_set
                    .read_volatile_at(i)
                    .portsc
                    .port_enabled_disabled()
                {}

                debug!(
                    "{TAG} Port {} reset, connected: {}",
                    i,
                    regs.port_register_set
                        .read_volatile_at(i)
                        .portsc
                        .current_connect_status()
                );
            }
        }
        info!("{TAG} Root hub init done.");

        self.dev_poll(0)?;

        Ok(())
    }

    fn dev_poll(&self, slot: u8) -> Result {
        let mut regs = self.regs.lock();
        // let changed =regs.operational.usbsts.read_volatile().port_change_detect();
        // if changed{
        //     regs.operational.usbsts.update_volatile(|r|{
        //         r.clear_port_change_detect();
        //     });
        // }else {
        //     return Ok(());
        // }
        debug!("{TAG} Poll slot {}", slot);

        let port_len = regs.port_register_set.len();

        for i in 0..port_len {
            regs.port_register_set.update_volatile_at(i, |port| {
                if port.portsc.connect_status_change() {
                    port.portsc.clear_connect_status_change();

                    debug!("{TAG} Port {i} status changed.");
                }
            });
        }


    


        Ok(())
    }

    fn probe(&self) -> Result {
        let mut regs = self.regs.lock();

        let port_len = regs.port_register_set.len();

        for i in 0..port_len {
            regs.port_register_set.update_volatile_at(i, |port| {
                // port.portsc.set_0_port_enabled_disabled();
                port.portsc.set_port_reset();
                while !port.portsc.port_reset() {}

                info!(
                    "{TAG} Port {}: Connected: {}, Speed {}, Power {}",
                    i,
                    port.portsc.current_connect_status(),
                    port.portsc.port_speed(),
                    port.portsc.port_power()
                );
            });
        }

        Ok(())
    }

    fn control(&self, direction: Direction, src: &mut [u8])->Result{

        




        Ok(())
    }
}
