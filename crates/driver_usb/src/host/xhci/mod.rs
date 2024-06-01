use crate::{addr::VirtAddr, err::*, OsDep};
use alloc::{borrow::ToOwned, format, vec::Vec};
use axhal::irq::IrqHandler;
use core::{
    alloc::Allocator,
    borrow::BorrowMut,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
};
use log::*;
use spinlock::SpinNoIrq;
use xhci::{
    context::{Input, InputHandler, Slot, Slot64Byte},
    registers::PortRegisterSet,
    ring::trb::{
        command::{AddressDevice, Allowed, EnableSlot, Noop},
        event::CommandCompletion,
    },
};

pub use xhci::ring::trb::transfer::Direction;
mod registers;
use registers::*;
mod context;
mod event;
pub(crate) mod ring;
use self::{context::*, event::EventRing, ring::Ring};
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
        let mut regs = Registers::new_registers(mmio_base);

        // TODO: pcie 未配置，读不出来
        // let version = self.regs.capability.hciversion.read_volatile();
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
    fn chip_hardware_reset(&mut self) -> Result {
        debug!("{TAG} Reset begin");
        debug!("{TAG} Stop");

        let mut g = self.regs.lock();
        let regs = &mut g.regs;

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

    fn set_max_device_slots(&self) -> Result {
        let mut regs = self.regs.lock();
        debug!("{TAG} Setting enabled slots to {}.", self.max_slots);
        regs.regs.operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(self.max_slots);
        });
        Ok(())
    }

    fn set_dcbaap(&self) -> Result {
        let dcbaap = { self.dev_ctx.lock().dcbaap() };
        let mut regs = self.regs.lock();
        debug!("{TAG} Writing DCBAAP: {:X}", dcbaap);
        regs.regs.operational.dcbaap.update_volatile(|r| {
            r.set(dcbaap as u64);
        });
        Ok(())
    }

    fn set_cmd_ring(&self) -> Result {
        let crcr = { self.ring.lock().register() };
        let mut regs = self.regs.lock();

        debug!("{TAG} Writing CRCR: {:X}", crcr);
        regs.regs.operational.crcr.update_volatile(|r| {
            r.set_command_ring_pointer(crcr);
            if (self.ring.lock().cycle) {
                r.set_ring_cycle_state();
            } else {
                r.clear_ring_cycle_state();
            }
        });

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
        let mut g = self.regs.lock();
        let regs = &mut g.regs;
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
        let mut g = self.regs.lock();
        let regs = &mut g.regs;

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

        // };

        // self.setup_scratchpads(buf_count);

        Ok(())
    }

    fn post_cmd(&self, mut trb: Allowed) -> Result<ring::trb::event::CommandCompletion> {
        {
            let mut cr = self.ring.lock();
            let addr = trb.as_ref().as_ptr().addr();
            if cr.cycle {
                trb.set_cycle_bit();
            } else {
                trb.clear_cycle_bit();
            }
            cr.enque_trb(trb.into_raw());

            debug!("{TAG} Post cmd {:?} @{:X}", trb, addr);

            let mut regs = self.regs.lock();

            regs.regs.doorbell.update_volatile_at(0, |r| {
                r.set_doorbell_stream_id(0);
                r.set_doorbell_target(0);
            });
        }

        O::force_sync_cache();

        debug!("{TAG} Wait result");
        {
            let mut er = self.primary_event_ring.lock();

            loop {
                let event = er.next();
                match event {
                    xhci::ring::trb::event::Allowed::CommandCompletion(c) => {
                        while c.completion_code().is_err() {}
                        debug!(
                            "{TAG} Cmd @{:X} got result, cycle {}",
                            c.command_trb_pointer(),
                            c.cycle_bit()
                        );

                        return Ok(c);
                    }
                    _ => warn!("event: {:?}", event),
                }
            }
        }
    }

    fn test_cmd(&self) -> Result {
        debug!("{TAG} Test command ring");
        for _ in 0..3 {
            let completion = self.post_cmd(Allowed::Noop(Noop::new()))?;
        }
        debug!("{TAG} Command ring ok");
        Ok(())
    }

    fn setup_scratchpads(&mut self) {
        let buf_count = {
            let regs = self.regs.lock();
            let count = regs
                .regs
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
        let mut g = self.regs.lock();
        let regs = &mut g.regs;
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
    fn reset_ports(&self) {
        let mut g = self.regs.lock();

        let regs = &mut g.regs;
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

    fn probe(&self) -> Result {
        let mut port_id_list = Vec::new();
        {
            let mut g = self.regs.lock();
            let regs = &mut g.regs;
            let port_len = regs.port_register_set.len();
            for i in 0..port_len {
                let portsc = &regs.port_register_set.read_volatile_at(i).portsc;
                info!(
                    "{TAG} Port {}: Enabled: {}, Connected: {}, Speed {}, Power {}",
                    i,
                    portsc.port_enabled_disabled(),
                    portsc.current_connect_status(),
                    portsc.port_speed(),
                    portsc.port_power()
                );

                if !portsc.port_enabled_disabled() {
                    continue;
                }

                port_id_list.push(i);
            }
        }
        for port_id in port_id_list {
            let slot = self.device_slot_assignment(port_id);
            self.address_device(slot, port_id);
        }

        Ok(())
    }

    fn get_psi(&self, port: usize) -> u8 {
        self.regs
            .lock()
            .regs
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

    fn address_device(&self, slot: u8, port: usize) -> Result {
        let index = self
            .dev_ctx
            .lock()
            .attached_set
            .get(&(slot as usize))
            .unwrap()
            .address;
        let mut binding = self.dev_ctx.lock();

        let transfer_ring_0_addr = binding
            .attached_set
            .get(&index)
            .unwrap()
            .transfer_rings
            .get(0)
            .unwrap()
            .register();

        let context_mut = binding
            .device_input_context_list
            .get_mut(index)
            .unwrap()
            .deref_mut();

        let control_context = context_mut.control_mut();
        control_context.set_add_context_flag(0);
        control_context.set_add_context_flag(1);

        let slot_context = context_mut.device_mut().slot_mut();
        slot_context.clear_multi_tt();
        slot_context.clear_hub();
        slot_context.set_context_entries(1);
        slot_context.set_max_exit_latency(0);
        slot_context.set_root_hub_port_number((port) as u8); //todo: to use port number
        slot_context.set_number_of_ports(0);
        slot_context.set_parent_hub_slot_id(0);
        slot_context.set_tt_think_time(0);
        slot_context.set_interrupter_target(0);

        let endpoint_0 = context_mut.device_mut().endpoint_mut(1);
        endpoint_0.set_error_count(3);
        endpoint_0.set_endpoint_type(xhci::context::EndpointType::Control);
        endpoint_0.set_host_initiate_disable();
        endpoint_0.set_max_burst_size(0);
        endpoint_0.set_max_packet_size(self.get_speed(port));
        endpoint_0.set_tr_dequeue_pointer(transfer_ring_0_addr);

        debug!("{TAG} CMD: address device");
        O::force_sync_cache();

        let result = self.post_cmd(Allowed::AddressDevice(
            *AddressDevice::new()
                .set_slot_id(slot)
                .set_input_context_pointer((context_mut as *const Input<16>).addr() as u64),
        ))?;

        debug!("{TAG} Result: {:?}", result);

        Ok(())
    }

    fn device_slot_assignment(&self, port: usize) -> u8 {
        // enable slot
        let mut cmd = EnableSlot::new();
        let slot_type = {
            // TODO: PCI未初始化，读不出来
            // let mut regs = self.regs.lock();
            // match regs.supported_protocol(port) {
            //     Some(p) => p.header.read_volatile().protocol_slot_type(),
            //     None => {
            //         warn!(
            //             "{TAG} Failed to find supported protocol information for port {}",
            //             port
            //         );
            //         0
            //     }
            // }
            0
        };
        cmd.set_slot_type(slot_type);

        let cmd = Allowed::EnableSlot(EnableSlot::new());

        debug!("{TAG} CMD: enable slot");

        let result = self.post_cmd(cmd).unwrap();

        let slot_id = result.slot_id();
        debug!("{TAG} Result: {:?}, slot id: {slot_id}", result);

        self.dev_ctx
            .lock()
            .new_slot(slot_id as usize, 0, port, 16)
            .unwrap(); //assume 16

        slot_id
    }

    fn control(&self, slot: u8, direction: Direction, src: &mut [u8]) -> Result {
        let slot = slot as usize;
        {
            let mut dev_ctx = self.dev_ctx.lock();

            let ctx = &mut dev_ctx.device_out_context_list[slot];
            let ep0 = ctx.endpoint_mut(0);
        }

        Ok(())
    }
}
