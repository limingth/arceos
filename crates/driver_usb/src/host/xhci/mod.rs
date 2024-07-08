use crate::{err::*, OsDep};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    rc::Rc,
    vec::{self, Vec},
};
use axalloc::global_no_cache_allocator;
use axhal::{cpu::this_cpu_is_bsp, irq::IrqHandler, paging::PageSize};
use core::{
    alloc::Allocator,
    borrow::BorrowMut,
    cell::RefCell,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    sync::atomic::{fence, Ordering},
};
use core::{
    cell::{Ref, RefMut},
    iter::Cycle,
};
use log::*;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::{
    context::{Input, InputHandler, Slot, Slot64Byte},
    registers::PortRegisterSet,
    ring::trb::{
        command::{self, AddressDevice, Allowed, EnableSlot, EvaluateContext, Noop},
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
    cmd: Ring<O>,
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
        let cmd = Ring::new(config.os.clone(), entries_per_page, true)?;
        let event = EventRing::new(config.os.clone())?;

        debug!("{TAG} ring size {}", cmd.len());

        let mut s = Self {
            config,
            max_slots,
            max_irqs,
            max_ports,
            scratchpad_buf_arr: None,
            cmd,
            event,
            regs,
            dev_ctx,
        };
        s.init()?;
        info!("{TAG} Init success");
        Ok(s)
    }

    fn poll(
        &mut self,
        arc: Arc<SpinNoIrq<Box<dyn Controller<O>>>>,
    ) -> Result<Vec<DeviceAttached<O>>> {
        let mut port_id_list = Vec::new();
        let port_len = self.regs().port_register_set.len();
        for i in 0..port_len {
            let portsc = &self.regs_mut().port_register_set.read_volatile_at(i).portsc;
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
        let mut device_list = Vec::new();
        for port_idx in port_id_list {
            let port_id = port_idx + 1;
            let slot = self.device_slot_assignment()?;
            let mut device = self
                .dev_ctx
                .new_slot(slot as usize, 0, port_id, 32, arc.clone())?;
            debug!("assign complete!");
            self.address_device(&device)?;
            
            // self.set_ep0_packet_size(slot);
            // debug!("packet size complete!");
            // self.setup_fetch_all_needed_dev_desc(slot);
            // debug!("fetch all complete!");

            device_list.push(device);
        }
        Ok(device_list)
    }

    fn post_cmd(&mut self, mut trb: command::Allowed) -> Result<CommandCompletion> {
        let addr = self.cmd.enque_command(trb);

        self.regs_mut().doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        fence(Ordering::Release);

        let r = self.event_busy_wait_next(addr as _)?;

        /// update erdp
        self.regs_mut()
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|f| {
                f.set_event_ring_dequeue_pointer(self.event.erdp());
            });

        Ok(r)
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

    fn device_slot_assignment(&mut self) -> Result<u8> {
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

        let result = self.post_cmd(cmd)?;

        let slot_id = result.slot_id();
        debug!("new slot id: {slot_id}");
        // self.dev_ctx.new_slot(slot_id as usize, 0, port_idx, 16).unwrap(); //assume 16
        Ok(slot_id)
    }
    pub fn address_device(&mut self, device: &DeviceAttached<O>) -> Result {
        let slot_id = device.slot_id;
        let port_idx = device.port_id - 1;
        let port_speed = self.get_psi(port_idx);
        let max_packet_size = self.get_speed(port_idx);

        let transfer_ring_0_addr = device.transfer_rings[0].register();
        let ring_cycle_bit = device.transfer_rings[0].cycle;
        let context_addr = {
            let context_mut = self
                .dev_ctx
                .device_input_context_list
                .get_mut(slot_id)
                .unwrap()
                .deref_mut();

            let control_context = context_mut.control_mut();
            control_context.set_add_context_flag(0);
            control_context.set_add_context_flag(1);
            for i in 2..32 {
                control_context.clear_drop_context_flag(i);
            }

            let slot_context = context_mut.device_mut().slot_mut();
            slot_context.clear_multi_tt();
            slot_context.clear_hub();
            slot_context.set_route_string(0); // for now, not support more hub ,so hardcode as 0.//TODO: generate route string
            slot_context.set_context_entries(1);
            slot_context.set_max_exit_latency(0);
            slot_context.set_root_hub_port_number(1); //todo: to use port number
            slot_context.set_number_of_ports(0);
            slot_context.set_parent_hub_slot_id(0);
            slot_context.set_tt_think_time(0);
            slot_context.set_interrupter_target(0);
            slot_context.set_speed(port_speed);

            let endpoint_0 = context_mut.device_mut().endpoint_mut(1);
            endpoint_0.set_endpoint_type(xhci::context::EndpointType::Control);
            endpoint_0.set_max_packet_size(max_packet_size);
            endpoint_0.set_max_burst_size(0);
            endpoint_0.set_error_count(3);
            endpoint_0.set_tr_dequeue_pointer(transfer_ring_0_addr);
            if ring_cycle_bit {
                endpoint_0.set_dequeue_cycle_state();
            } else {
                endpoint_0.clear_dequeue_cycle_state();
            }
            endpoint_0.set_interval(0);
            endpoint_0.set_max_primary_streams(0);
            endpoint_0.set_mult(0);
            endpoint_0.set_error_count(3);
            endpoint_0.set_average_trb_length(8);

            (context_mut as *const Input<16>).addr() as u64
        };

        fence(Ordering::Release);

        let result = self.post_cmd(Allowed::AddressDevice(
            *AddressDevice::new()
                .set_slot_id(slot_id as _)
                .set_input_context_pointer(context_addr),
        ))?;

        debug!("address slot [{}] ok", slot_id);

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

    fn event_busy_wait_next(&mut self, addr: u64) -> Result<CommandCompletion> {
        debug!("Wait result");
        loop {
            if let Some((event, cycle)) = self.event.next() {
                match event {
                    xhci::ring::trb::event::Allowed::CommandCompletion(c) => {
                        let mut code = CompletionCode::Invalid;
                        if let Ok(c) = c.completion_code() {
                            code = c;
                        } else {
                            continue;
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
        let crcr = self.cmd.register();
        let cycle = self.cmd.cycle;

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
