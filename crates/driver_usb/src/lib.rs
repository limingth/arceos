#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]
#![feature(auto_traits)]
#![feature(btreemap_alloc)]
#![feature(if_let_guard)]
#![feature(get_many_mut)]
#![feature(let_chains)]
#![feature(cfg_match)]
#![feature(iter_collect_into)]
#![feature(const_trait_impl)]

use core::{mem::MaybeUninit, usize};

use abstractions::{dma::DMA, PlatformAbstractions};
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
    vec::Vec,
};
use glue::driver_independent_device_instance::DriverIndependentDeviceInstance;
use host::{data_structures::MightBeInited, USBHostSystem};
use log::{error, trace};
use spinlock::SpinNoIrq;
use usb::{
    descriptors::{
        construct_control_transfer_type, parser::RawDescriptorParser,
        topological_desc::TopologicalUSBDescriptorRoot, USBStandardDescriptorTypes,
    },
    operation,
    trasnfer::control::{bmRequestType, ControlTransfer, DataTransferType, StandardbRequest},
    urb::{RequestedOperation, URB},
    USBDriverSystem,
};
use xhci::ring::trb::transfer::{Direction, TransferType};

extern crate alloc;

pub mod abstractions;
pub mod err;
pub mod glue;
pub mod host;
pub mod usb;

#[derive(Clone, Debug)]
pub struct USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub(crate) base_addr: O::VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) os: O,
}

pub struct USBSystem<'a, O>
where
    O: PlatformAbstractions,
{
    platform_abstractions: O,
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    host_driver_layer: USBHostSystem<O>,
    usb_driver_layer: USBDriverSystem<'a, O>,
    driver_independent_devices: Vec<DriverIndependentDeviceInstance<O>>,
}

impl<'a, O> USBSystem<'a, O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: USBSystemConfig<O>) -> Self {
        let config = Arc::new(SpinNoIrq::new(config));
        Self {
            config: config.clone(),
            platform_abstractions: config.clone().lock().os.clone(),
            host_driver_layer: USBHostSystem::new(config.clone()).unwrap(),
            usb_driver_layer: USBDriverSystem::new(config.clone()),
            driver_independent_devices: Vec::new(),
        }
    }

    pub fn init(mut self) -> Self {
        trace!("initializing!");
        self.host_driver_layer.init();
        self.usb_driver_layer.init();
        trace!("usb system init complete");
        self
    }

    pub fn init_probe(mut self) -> Self {
        // async { //todo:async it!
        {
            self.driver_independent_devices.clear(); //need to have a merge algorithm for hot plug
            let mut after = Vec::new();
            self.host_driver_layer.probe(|device| after.push(device));

            for driver in after {
                self.new_device(driver)
            }
            trace!("device probe complete");
        }
        {
            let mut preparing_list = Vec::new();
            self.usb_driver_layer
                .init_probe(&mut self.driver_independent_devices, &mut preparing_list);

            //probe driver modules and load them
            self.host_driver_layer
                .tock(preparing_list, &mut self.driver_independent_devices);

            //and do some prepare stuff
        }
        // }
        // .await;

        self
    }

    pub fn init_probe1(mut self) -> Self {
        // async { //todo:async it!
        {
            self.driver_independent_devices.clear(); //need to have a merge algorithm for hot plug
            let mut after = Vec::new();
            self.host_driver_layer.probe(|device| after.push(device));

            for driver in after {
                self.new_device(driver)
            }
            trace!("device probe complete");
        }
        {
            let mut preparing_list = Vec::new();
            self.usb_driver_layer
                .init_probe1(&mut self.driver_independent_devices, &mut preparing_list);

            //probe driver modules and load them
            self.host_driver_layer
                .tock(preparing_list, &mut self.driver_independent_devices);

            //and do some prepare stuff
        }
        // }
        // .await;

        self
    }

    pub fn driver_active(mut self) -> Self {
        self
    }

    pub fn drive_all(mut self) -> Self {
        //TODO: Drive All

        loop {
            let tick = self.usb_driver_layer.tick();
            if tick.len() != 0 {
                trace!("tick! {:?}", tick.len());
                self.host_driver_layer
                    .tock(tick, &mut self.driver_independent_devices);
            }
            // trace!("tock!");
        }
        self
    }

    pub fn drive_all1(mut self) -> Self {
        //TODO: Drive All

        loop {
            let tick = self.usb_driver_layer.tick1();
            if tick.len() != 0 {
                trace!("tick! {:?}", tick.len());
                self.host_driver_layer
                    .tock(tick, &mut self.driver_independent_devices);
            }
            // trace!("tock!");
        }
        self
    }

    pub fn drive_all2(mut self) -> Self {
        //TODO: Drive All

        loop {
            let tick = self.usb_driver_layer.tick2();
            if tick.len() != 0 {
                trace!("tick! {:?}", tick.len());
                self.host_driver_layer
                    .tock(tick, &mut self.driver_independent_devices);
            }
            // trace!("tock!");
        }
        self
    }

    pub fn drop_device(&mut self, mut driver_independent_device_slot_id: usize) {
        //do something
    }

    pub fn new_device(&mut self, mut driver: DriverIndependentDeviceInstance<O>) {
        'label: {
            if let MightBeInited::Uninit = *driver.descriptors {
                let buffer_device = DMA::new_vec(
                    0u8,
                    O::PAGE_SIZE,
                    O::PAGE_SIZE,
                    self.config.lock().os.dma_alloc(),
                );

                let desc = match (&driver.controller).lock().control_transfer(
                    driver.slotid,
                    ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Standard,
                            usb::trasnfer::control::Recipient::Device,
                        ),
                        request: StandardbRequest::GetDescriptor.into(),
                        index: 0,
                        value: construct_control_transfer_type(
                            USBStandardDescriptorTypes::Device as u8,
                            0,
                        )
                        .bits(),
                        data: Some(buffer_device.addr_len_tuple()),
                    },
                ) {
                    Ok(_) => {
                        let mut parser = RawDescriptorParser::<O>::new(buffer_device);
                        parser.single_state_cycle();
                        let num_of_configs = parser.num_of_configs();
                        for index in 0..num_of_configs {
                            let buffer = DMA::new_vec(
                                0u8,
                                O::PAGE_SIZE,
                                O::PAGE_SIZE,
                                self.config.lock().os.dma_alloc(),
                            );
                            (&driver.controller)
                                .lock()
                                .control_transfer(
                                    driver.slotid,
                                    ControlTransfer {
                                        request_type: bmRequestType::new(
                                            Direction::In,
                                            DataTransferType::Standard,
                                            usb::trasnfer::control::Recipient::Device,
                                        ),
                                        request: StandardbRequest::GetDescriptor.into(),
                                        index: 0,
                                        value: construct_control_transfer_type(
                                            USBStandardDescriptorTypes::Configuration as u8,
                                            index as _,
                                        )
                                        .bits(),
                                        data: Some(buffer.addr_len_tuple()),
                                    },
                                )
                                .inspect(|_| {
                                    parser.append_config(buffer);
                                });
                        }
                        driver.descriptors = Arc::new(MightBeInited::Inited(parser.summarize()));
                    }
                    Err(err) => {
                        error!("err! {:?}", err);
                        break 'label;
                    }
                };
            }

            trace!("parsed descriptor:{:#?}", driver.descriptors);

            if let MightBeInited::Inited(TopologicalUSBDescriptorRoot {
                device: devices,
                others,
                metadata,
            }) = &*driver.descriptors
            {
                self.host_driver_layer
                    .urb_request(
                        URB::new(
                            driver.slotid,
                            RequestedOperation::ConfigureDevice(
                                operation::Configuration::SetupDevice(
                                    //TODO: fixme
                                    devices.first().unwrap().child.first().unwrap(),
                                ),
                            ),
                        ),
                        &mut self.driver_independent_devices,
                    )
                    .unwrap();
            };

            self.driver_independent_devices.push(driver);
            trace!(
                "pushed new device! {:?}",
                self.driver_independent_devices.len()
            )
        }
        //do something
    }
}

// #[cfg(feature = "arceos")]
// pub mod ax;
