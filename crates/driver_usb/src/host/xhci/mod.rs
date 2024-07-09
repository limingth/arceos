use crate::{
    addr::VirtAddr,
    ax::USBDeviceDriverOps,
    dma::DMA,
    err::*,
    host::usb::{
        descriptors::RawDescriptorParser, drivers::driver_usb_hid::USBDeviceDriverHidMouseExample,
    },
    OsDep,
};
use alloc::{borrow::ToOwned, format, vec, vec::Vec};
use axalloc::global_no_cache_allocator;
use axhal::{cpu::this_cpu_is_bsp, irq::IrqHandler, paging::PageSize};
use core::{
    alloc::Allocator, borrow::BorrowMut, iter::Cycle, num::NonZeroUsize, ops::{Deref, DerefMut}, sync::atomic::{fence, Ordering}
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

pub static mut drivers: Option<Arc<SpinNoIrq<USBDeviceDriverHidMouseExample>>> = None;

pub struct Xhci<O>
where
    O: OsDep,
{
    pub(super) config: USBHostConfig<O>,
    pub(super) regs: SpinNoIrq<Registers>,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    pub(super) dev_ctx: SpinNoIrq<DeviceContextList<O>>,
    pub(super) ring: SpinNoIrq<Ring<O>>,
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

    pub fn post_cmd(&self, mut trb: Allowed) -> Result<ring::trb::event::CommandCompletion> {
        {
            let mut cr = self.ring.lock();
            if cr.cycle {
                trb.set_cycle_bit();
            } else {
                trb.clear_cycle_bit();
            }
            let addr = cr.enque_trb(trb.into_raw());

            debug!("{TAG} Post cmd {:?} @{:X}", trb, addr);

            let mut regs = self.regs.lock();

            regs.regs.doorbell.update_volatile_at(0, |r| {
                r.set_doorbell_stream_id(0);
                r.set_doorbell_target(0);
            });
        }

        fence(Ordering::Release);

        debug!("{TAG} Wait result");
        {
            let mut er = self.primary_event_ring.lock();

            loop {
                let event = er.next();
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
                            "{TAG} Cmd @{:X} got result, cycle {}",
                            c.command_trb_pointer(),
                            c.cycle_bit()
                        );
                        if let CompletionCode::Success = code {
                            return Ok(c);
                        }
                        return Err(Error::Unknown(format!("{:?}", code)));
                    }
                    _ => warn!("event: {:?}", event),
                }
            }
        }
    }

    pub fn post_control_transfer_with_data(
        &self,
        (setup, data, status): (transfer::Allowed, transfer::Allowed, transfer::Allowed),
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        self.post_control_transfer(vec![setup, data, status], transfer_ring, dci, slot_id)
    }

    pub fn post_control_transfer_with_data_and_busy_wait(
        &self,
        (setup, data, status): (transfer::Allowed, transfer::Allowed, transfer::Allowed),
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        self.post_control_transfer_and_busy_wait(
            vec![setup, data, status],
            transfer_ring,
            dci,
            slot_id,
        )
    }

    fn post_control_transfer_and_busy_wait(
        &self,
        mut transfer_trbs: Vec<transfer::Allowed>,
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        let collect = transfer_trbs
            .iter_mut()
            .map(|trb| {
                if self.ring.lock().cycle {
                    debug!("{TAG} Setting cycle bit for TRB {:?}", trb);
                    trb.set_cycle_bit();
                } else {
                    debug!("{TAG} Setting cycle bit for TRB {:?}", trb);
                    trb.clear_cycle_bit();
                }
                trb.into_raw()
            })
            .collect();
        transfer_ring.enque_trbs(collect);
        debug!("{TAG} Post control transfer!");

        let mut regs = self.regs.lock();

        regs.regs.doorbell.update_volatile_at(slot_id, |r| {
            r.set_doorbell_target(dci);
        });

        O::force_sync_cache();

        debug!("{TAG} Wait result");
        self.busy_wait_for_event()
    }

    pub fn post_control_transfer_no_data_and_busy_wait(
        &self,
        (setup, status): (transfer::Allowed, transfer::Allowed),
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        self.post_control_transfer(vec![setup, status], transfer_ring, dci, slot_id)
    }

    pub fn post_transfer_not_control(
        &self,
        request: transfer::Allowed,
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        self.post_control_transfer(vec![request], transfer_ring, dci, slot_id)
    }

    fn post_control_transfer(
        &self,
        mut transfer_trbs: Vec<transfer::Allowed>,
        transfer_ring: &mut Ring<O>,
        dci: u8,
        slot_id: usize,
    ) -> Result<ring::trb::event::TransferEvent> {
        let collect = transfer_trbs
            .iter_mut()
            .map(|trb| {
                if self.ring.lock().cycle {
                    trb.set_cycle_bit();
                } else {
                    trb.clear_cycle_bit();
                }
                trb.into_raw()
            })
            .collect();
        transfer_ring.enque_trbs(collect);

        debug!("{TAG} Post control transfer!");

        let mut regs = self.regs.lock();

        regs.regs.doorbell.update_volatile_at(slot_id, |r| {
            r.set_doorbell_target(dci);
        });

        fence(Ordering::Release);

        self.busy_wait_for_event()
    }

    pub fn busy_wait_for_event(&self) -> Result<ring::trb::event::TransferEvent> {
        debug!("{TAG} Wait result");
        {
            let mut er = self.primary_event_ring.lock();

            loop {
                if let Some(temp) = er.busy_wait_next() {
                    debug!("received temp!:{:?}", temp);
                    let event = er.next();
                    match event {
                        xhci::ring::trb::event::Allowed::TransferEvent(c) => {
                            while c.completion_code().is_err() {}
                            debug!(
                                "{TAG} Transfer @{:X} got result, cycle {}",
                                c.trb_pointer(),
                                c.cycle_bit()
                            );

                            return Ok(c);
                        }
                        _ => warn!("event: {:?}", event),
                    }
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
            debug!("assign complete!");
            self.address_device(slot, port_id);
            debug!("address complete!");
            self.set_ep0_packet_size(slot);
            debug!("packet size complete!");
            self.setup_fetch_all_needed_dev_desc(slot);
            debug!("fetch all complete!");
        }

        let mut lock = self.dev_ctx.lock();
        let dev_ctx_list = (&mut lock.device_input_context_list as *mut Vec<_>);
        lock.attached_set.iter_mut().for_each(|dev| {
            debug!("set cfg!");
            dev.1.set_configuration(
                FromPrimitive::from_u8(
                    self.regs
                        .lock()
                        .regs
                        .port_register_set
                        .read_volatile_at(dev.1.port)
                        .portsc
                        .port_speed()
                        .into(),
                )
                .unwrap(),
                |allowed| self.post_cmd(allowed),
                |allowed, ring, dci, slot| {
                    self.post_control_transfer_no_data_and_busy_wait(allowed, ring, dci, slot)
                },
                |request_type, request, value, index, transfer_type| {
                    self.construct_no_data_transfer_req(
                        request_type,
                        request,
                        value,
                        index,
                        transfer_type,
                    )
                },
                (unsafe { &mut *dev_ctx_list }), //ugly!
            );
        });
        // debug!("attached count: {}", lock.attached_set.len());
        // lock.attached_set.iter_mut().for_each(|dev| {
        //     debug!("find driver!");
        //     let find_driver_impl = dev.1.find_driver_impl::<USBDeviceDriverHidMouseExample>();
        //     if let Some(driver) = find_driver_impl {
        //         debug!("found!");
        //         <USBDeviceDriverHidMouseExample as USBDeviceDriverOps<O>>::work(
        //             //should create a task
        //             &driver.lock(),
        //             self,
        //         );
        //     }
        // })

        let dev = lock.attached_set.get_mut(&1).unwrap(); //从这里开始是实验环节
        unsafe {
            drivers = Some(
                dev.find_driver_impl::<USBDeviceDriverHidMouseExample>()
                    .unwrap(),
            )
        };

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

    fn setup_fetch_all_needed_dev_desc(&self, slot: u8) -> Result {
        //todo fetch all desc
        let mut binding = self.dev_ctx.lock();
        let mut dev = binding.attached_set.get_mut(&(slot as usize)).unwrap();

        self.fetch_device_desc(dev, slot);
        self.fetch_config_desc(dev, slot);

        debug!("fetched descriptors:{:#?}", dev.descriptors);
        Ok(())
    }

    fn fetch_config_desc(&self, dev: &mut xhci_device::DeviceAttached<O>, slot: u8) {
        let buffer = DMA::new_vec(
            0u8,
            PageSize::Size4K.into(),
            PageSize::Size4K.into(),
            self.config.os.dma_alloc(),
        );
        let construct_control_transfer_req = self.construct_control_transfer_req(
            &buffer,
            0b1000_0000,
            6u8,
            descriptors::DescriptorType::Configuration.forLowBit(0),
            0,
            (TransferType::In, Direction::In),
        );
        debug!("{TAG} Transfer Control: Fetching config desc");
        let post_control_transfer = self
            .post_control_transfer_with_data(
                construct_control_transfer_req,
                dev.transfer_rings.get_mut(0).unwrap(),
                1,
                slot as usize,
            )
            .unwrap();
        debug!("{TAG} Result: {:?}", post_control_transfer);
        RawDescriptorParser::<O>::new(buffer).parse(&mut dev.descriptors);
    }

    fn fetch_device_desc(&self, dev: &mut xhci_device::DeviceAttached<O>, slot: u8) {
        let buffer = DMA::new_vec(
            0u8,
            PageSize::Size4K.into(),
            PageSize::Size4K.into(),
            self.config.os.dma_alloc(),
        );
        let construct_control_transfer_req = self.construct_control_transfer_req(
            &buffer,
            0b1000_0000,
            6u8,
            descriptors::DescriptorType::Device.forLowBit(0),
            0,
            (TransferType::In, Direction::In),
        );
        debug!("{TAG} Transfer Control: Fetching device desc");
        let post_control_transfer = self
            .post_control_transfer_with_data(
                construct_control_transfer_req,
                dev.transfer_rings.get_mut(0).unwrap(),
                1,
                slot as usize,
            )
            .unwrap();
        debug!("{TAG} Result: {:?}", post_control_transfer);
        RawDescriptorParser::<O>::new(buffer).parse(&mut dev.descriptors);
    }

    fn set_ep0_packet_size(&self, slot: u8) -> Result {
        let buffer = DMA::new_singleton_page4k(
            descriptors::desc_device::Device::default(),
            self.config.os.dma_alloc(),
        );
        let mut binding = self.dev_ctx.lock();
        let dev = binding.attached_set.get_mut(&(slot as usize)).unwrap();
        let index = dev.slot_id - 1;
        let transfer = self.construct_control_transfer_req(
            &buffer,
            0x80,
            6,
            descriptors::DescriptorType::Device.forLowBit(0),
            0,
            (TransferType::In, Direction::In),
        );

        debug!("{TAG} CMD: get endpoint0 packet size");
        let command_completion = self.post_control_transfer_with_data(
            transfer,
            &mut dev.transfer_rings.get_mut(0).unwrap(),
            1, //TODO: calculate dci
            slot as usize,
        )?;
        debug!("{TAG} Result: {:?}", command_completion);

        let max_packet_size = buffer.max_packet_size();

        let input = binding
            .device_input_context_list
            .get_mut(index)
            .unwrap()
            .deref_mut();
        input
            .device_mut()
            .endpoint_mut(1) //dci=1: endpoint 0
            .set_max_packet_size(max_packet_size);

        debug!(
            "{TAG} CMD: evaluating context for set endpoint0 packet size {}",
            max_packet_size
        );
        let eval_ctx = self.post_cmd(Allowed::EvaluateContext(
            *EvaluateContext::default()
                .set_slot_id(slot)
                .set_input_context_pointer((input as *mut Input<16>).addr() as u64),
        ))?;
        debug!("{TAG} Result: {:?}", eval_ctx);

        Ok(())
    }

    fn address_device(&self, slot: u8, port: usize) -> Result {
        let slot_id = slot as usize;

        let mut binding = self.dev_ctx.lock();

        let transfer_ring_0_addr = binding
            .attached_set
            .get(&slot_id)
            .unwrap()
            .transfer_rings
            .get(0)
            .unwrap()
            .register();
        let ring_cycle_bit = binding
            .attached_set
            .get(&slot_id)
            .unwrap()
            .transfer_rings
            .get(0)
            .unwrap()
            .cycle;

        let context_mut = binding
            .device_input_context_list
            .get_mut(slot_id - 1)
            .unwrap()
            .deref_mut();

        let control_context = context_mut.control_mut();
        control_context.set_add_context_flag(0);
        control_context.set_add_context_flag(1);

        let slot_context = context_mut.device_mut().slot_mut();
        slot_context.clear_multi_tt();
        slot_context.clear_hub();
        slot_context.set_route_string(0); // for now, not support more hub ,so hardcode as 0.//TODO: generate route string
        slot_context.set_context_entries(1);
        slot_context.set_max_exit_latency(0);
        slot_context.set_root_hub_port_number((port + 1) as u8); //todo: to use port number
        slot_context.set_number_of_ports(0);
        slot_context.set_parent_hub_slot_id(0);
        slot_context.set_tt_think_time(0);
        slot_context.set_interrupter_target(0);
        slot_context.set_speed(self.get_psi(port));

        let endpoint_0 = context_mut.device_mut().endpoint_mut(1);
        endpoint_0.set_endpoint_type(xhci::context::EndpointType::Control);
        endpoint_0.set_max_packet_size(self.get_speed(port));
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

        fence(Ordering::Release);

        let result = self.post_cmd(Allowed::AddressDevice(
            *AddressDevice::new()
                .set_slot_id(slot)
                .set_input_context_pointer((context_mut as *const Input<16>).addr() as u64),
        ))?;

        debug!("address [{}] ok", slot_id);

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

        let mut lock = self.dev_ctx.lock();
        debug!("new slot!");
        lock.new_slot(slot_id as usize, 0, port, 16).unwrap(); //assume 16

        slot_id
    }

    pub fn construct_no_data_transfer_req(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        transfer_type: TransferType,
    ) -> (transfer::Allowed, transfer::Allowed) {
        let setup = *transfer::SetupStage::default()
            .set_request_type(request_type)
            .set_request(request) //get_desc
            .set_value(value)
            .set_length(0)
            .set_transfer_type(transfer_type)
            .set_index(index);
        let status = *transfer::StatusStage::default().set_interrupt_on_completion();

        (setup.into(), status.into())
    }

    pub fn construct_control_transfer_req<T: ?Sized>(
        &self,
        buffer: &DMA<T, O::DMA>,
        request_type: u8,
        request: u8,
        value: descriptors::DescriptionTypeIndexPairForControlTransfer,
        index: u16,
        transfertype_direction: (TransferType, Direction),
    ) -> (transfer::Allowed, transfer::Allowed, transfer::Allowed) {
        let setup = *transfer::SetupStage::default()
            .set_request_type(request_type)
            .set_request(request) //get_desc
            .set_value(value.bits())
            .set_length(buffer.length_for_bytes().try_into().unwrap())
            .set_transfer_type(transfertype_direction.0)
            .set_index(index);

        let data = *transfer::DataStage::default()
            .set_data_buffer_pointer(buffer.addr() as u64)
            .set_trb_transfer_length(buffer.length_for_bytes().try_into().unwrap())
            .set_direction(transfertype_direction.1);

        let status = *transfer::StatusStage::default().set_interrupt_on_completion();

        (setup.into(), data.into(), status.into())
    }
}