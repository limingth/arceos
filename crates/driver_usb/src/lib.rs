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

use core::usize;

use abstractions::PlatformAbstractions;
use alloc::{
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
    vec::Vec,
};
use glue::driver_independent_device_instance::DriverIndependentDeviceInstance;
use host::USBHostSystem;
use spinlock::SpinNoIrq;
use usb::USBDriverSystem;

extern crate alloc;

pub mod abstractions;
pub mod err;
pub mod glue;
pub mod host;
pub mod usb;

#[derive(Clone)]
pub struct USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub(crate) base_addr: O::VirtAddr,
    pub(crate) irq_num: u32,
    pub(crate) irq_priority: u32,
    pub(crate) os: O,
}

pub struct USBSystem<O>
where
    O: PlatformAbstractions,
{
    platform_abstractions: O,
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    host_driver_layer: USBHostSystem<O>,
    usb_driver_layer: USBDriverSystem,
    driver_independent_devices: Vec<DriverIndependentDeviceInstance<O>>,
}

impl<O> USBSystem<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: USBSystemConfig<O>) -> Self {
        let config = Arc::new(SpinNoIrq::new(config));
        Self {
            config: config.clone(),
            platform_abstractions: config.clone().lock().os.clone(),
            host_driver_layer: USBHostSystem::new(config.clone()).unwrap(),
            usb_driver_layer: USBDriverSystem,
            driver_independent_devices: Vec::new(),
        }
    }

    pub fn init(self) -> Self {
        self.host_driver_layer.init();
        self.usb_driver_layer.init();
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
        }

        {
            self.usb_driver_layer.init_probe(); //probe driver modules and load them
        }
        // }
        // .await;
        self
    }

    pub fn drop_device(&mut self, driver_independent_device_slot_id: usize) {
        //do something
    }

    pub fn new_device(&mut self, driver: DriverIndependentDeviceInstance<O>) {
        self.driver_independent_devices.push(driver);
        //do something
    }
}

// #[cfg(feature = "arceos")]
// pub mod ax;
