use crate::{
    err::*,
    OsDep,
};
use alloc::{
    borrow::ToOwned,
    format,
    rc::Rc,
    vec::{self, Vec},
};
use axalloc::global_no_cache_allocator;
use axhal::{cpu::this_cpu_is_bsp, irq::IrqHandler, paging::PageSize};
use core::cell::{Ref, RefMut};
use core::{
    alloc::Allocator,
    borrow::BorrowMut,
    cell::RefCell,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    sync::atomic::{fence, Ordering},
};
use log::*;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::{
    context::{Input, InputHandler, Slot, Slot64Byte},
    registers::PortRegisterSet,
    ring::trb::{
        command::{AddressDevice, Allowed, EnableSlot, EvaluateContext, Noop},
        event::{CommandCompletion, CompletionCode},
        transfer::{self, DataStage, SetupStage, StatusStage, TransferType},
    },
};
use xhci_device::DeviceAttached;

pub use xhci::ring::trb::transfer::Direction;
mod registers;
use registers::*;
mod context;
mod event;
pub(crate) mod ring;
pub(crate) mod xhci_device;
use self::{context::*, event::EventRing, ring::Ring};
use super::{
    usb::{self, descriptors},
    Controller, USBHostConfig,
};
use crate::host::device::*;
use alloc::sync::Arc;
use core::mem;
const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;
const TAG: &str = "[XHCI]";

// pub static mut drivers: Option<Arc<SpinNoIrq<USBDeviceDriverHidMouseExample>>> = None;

pub struct Xhci<O>
where
    O: OsDep,
{
    pub(super) config: USBHostConfig<O>,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    scratchpad_buf_arr: Option<ScratchpadBufferArray<O>>,
    ring: Ring<O>,
    event: EventRing<O>,
    regs: Registers,
    pub dev_ctx: DeviceContextList<O>,
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
        let mut regs = Registers::new_registers(mmio_base);

        // TODO: pcie 未配置，读不出来
        // let version = self.core_mut().regs.capability.hciversion.read_volatile();
        // info!("xhci version: {:x}", version.get());
        let hcsp1 = regs.regs.capability.hcsparams1.read_volatile();
        let max_slots = hcsp1.number_of_device_slots();
        let max_ports = hcsp1.number_of_ports();
        let max_irqs = hcsp1.number_of_interrupts();
        let page_size = regs.regs.operational.pagesize.read_volatile().get();
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
            max_slots,
            max_irqs,
            max_ports,
            scratchpad_buf_arr: None,
            ring,
            event,
            regs,
            dev_ctx,
        };
        s.init()?;
        info!("{TAG} Init success");
        Ok(s)
    }

    fn poll(&mut self) -> Result {
        self.probe()
    }
}

impl<O> Xhci<O>
where
    O: OsDep,
{
    fn init(&mut self) -> Result {
        self.chip_hardware_reset()?;
        self.set_max_device_slots()?;
        self.set_dcbaap()?;
        self.set_cmd_ring()?;
        self.init_ir()?;

        self.setup_scratchpads();
        self.start()?;

        self.test_cmd()?;
        self.reset_ports();
        Ok(())
    }

    fn test_cmd(&mut self) -> Result {
        debug!("{TAG} Test command ring");
        for _ in 0..3 {
            let completion = self.post_cmd(Allowed::Noop(Noop::new()))?;
        }
        debug!("{TAG} Command ring ok");
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

    fn probe(&self) -> Result {
        






        Ok(())
    }

    fn setup_scratchpads(&mut self) {
        let scratchpad_buf_arr = {
            let buf_count = {
                let count = self
                    .regs()
                    .capability
                    .hcsparams2
                    .read_volatile()
                    .max_scratchpad_buffers();
                debug!("{TAG} Scratch buf count: {}", count);
                count
            };
            if buf_count == 0 {
                return;
            }
            let scratchpad_buf_arr = ScratchpadBufferArray::new(buf_count, self.config.os.clone());

            self.dev_ctx.dcbaa[0] = scratchpad_buf_arr.register() as u64;

            debug!(
                "{TAG} Setting up {} scratchpads, at {:#0x}",
                buf_count,
                scratchpad_buf_arr.register()
            );
            scratchpad_buf_arr
        };

        self.scratchpad_buf_arr = Some(scratchpad_buf_arr);
    }
    pub fn post_cmd(&mut self, mut trb: Allowed) -> Result<CommandCompletion> {
        if self.ring.cycle {
            trb.set_cycle_bit();
        } else {
            trb.clear_cycle_bit();
        }
        let addr = self.ring.enque_trb(trb.into_raw()) as u64;

        debug!("[CMD] >> {:?} @{:X}", trb, addr);

        self.regs_mut().doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        fence(Ordering::Release);

        self.event_busy_wait_next(addr)
    }

    fn event_busy_wait_next(&mut self, addr: u64) -> Result<CommandCompletion> {
        debug!("Wait result");
        loop {
            let event = self.event.next();
            match event {
                xhci::ring::trb::event::Allowed::CommandCompletion(c) => {
                    let mut code = CompletionCode::Invalid;
                    loop {
                        if let Ok(c) = c.completion_code() {
                            code = c;
                            break;
                        }
                    }
                    debug!(
                        "[CMD] << {code:#?} @{:X} got result, cycle {}",
                        c.command_trb_pointer(),
                        c.cycle_bit()
                    );
                    if c.command_trb_pointer() != addr {
                        continue;
                    }

                    if let CompletionCode::Success = code {
                        return Ok(c);
                    }
                    return Err(Error::CMD(code));
                }
                _ => warn!("event: {:?}", event),
            }
        }
    }

    fn regs(&self) -> &RegistersBase {
        &self.regs.regs
    }

    fn regs_mut(&mut self) -> &mut RegistersBase {
        &mut self.regs.regs
    }

    fn chip_hardware_reset(&mut self) -> Result {
        debug!("{TAG} Reset begin");
        debug!("{TAG} Stop");

        self.regs_mut().operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} Until halt");
        while !self.regs().operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} Halted");

        let mut o = &mut self.regs_mut().operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("{TAG} Wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        debug!("{TAG} Ready");

        o.usbcmd.update_volatile(|f| {
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        debug!("{TAG} Reset HC");

        while self
            .regs()
            .operational
            .usbcmd
            .read_volatile()
            .host_controller_reset()
            || self
                .regs()
                .operational
                .usbsts
                .read_volatile()
                .controller_not_ready()
        {}

        info!("{TAG} XCHI reset ok");
        Ok(())
    }

    fn set_max_device_slots(&mut self) -> Result {
        let max_slots = self.max_slots;
        debug!("{TAG} Setting enabled slots to {}.", max_slots);
        self.regs_mut().operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(max_slots);
        });
        Ok(())
    }

    fn set_dcbaap(&mut self) -> Result {
        let dcbaap = self.dev_ctx.dcbaap();
        debug!("{TAG} Writing DCBAAP: {:X}", dcbaap);
        self.regs_mut().operational.dcbaap.update_volatile(|r| {
            r.set(dcbaap as u64);
        });
        Ok(())
    }

    fn set_cmd_ring(&mut self) -> Result {
        let crcr = self.ring.register();
        let cycle = self.ring.cycle;

        let regs = self.regs_mut();

        debug!("{TAG} Writing CRCR: {:X}", crcr);
        regs.operational.crcr.update_volatile(|r| {
            r.set_command_ring_pointer(crcr);
            if cycle {
                r.set_ring_cycle_state();
            } else {
                r.clear_ring_cycle_state();
            }
        });

        Ok(())
    }

    fn start(&mut self) -> Result {
        let regs = self.regs_mut();
        debug!("{TAG} Start run");
        regs.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} Is running");

        regs.doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        Ok(())
    }

    fn init_ir(&mut self) -> Result {
        debug!("{TAG} Disable interrupts");
        let regs = &mut self.regs.regs;

        regs.operational.usbcmd.update_volatile(|r| {
            r.clear_interrupter_enable();
        });

        let mut ir0 = regs.interrupter_register_set.interrupter_mut(0);
        {
            debug!("{TAG} Writing ERSTZ");
            ir0.erstsz.update_volatile(|r| r.set(1));

            let erdp = self.event.erdp();
            debug!("{TAG} Writing ERDP: {:X}", erdp);

            ir0.erdp.update_volatile(|r| {
                r.set_event_ring_dequeue_pointer(erdp);
            });

            let erstba = self.event.erstba();
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

        // };

        // self.setup_scratchpads(buf_count);

        Ok(())
    }

    fn get_psi(&self, port: usize) -> u8 {
        self.regs()
            .port_register_set
            .read_volatile_at(port)
            .portsc
            .port_speed()
    }

    fn get_speed(&self, port: usize) -> u16 {
        match self.get_psi(port) {
            1 | 3 => 64,
            2 => 8,
            4 => 512,
            v => unimplemented!("PSI: {}", v),
        }
    }

    fn reset_cic(&mut self) {
        let regs = self.regs_mut();
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

    fn reset_ports(&mut self) {
        let regs = self.regs_mut();
        let port_len = regs.port_register_set.len();

        for i in 0..port_len {
            debug!("{TAG} Port {} start reset", i,);
            regs.port_register_set.update_volatile_at(i, |port| {
                port.portsc.set_0_port_enabled_disabled();
                port.portsc.set_port_reset();
            });

            while regs
                .port_register_set
                .read_volatile_at(i)
                .portsc
                .port_reset()
            {}

            debug!("{TAG} Port {} reset ok", i);
        }
    }
}
