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

use abstractions::PlatformAbstractions;
use alloc::sync::Arc;
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
        }
    }

    pub fn init(self) -> Self {
        self.host_driver_layer.init();
        self.usb_driver_layer.init();
        self
    }

    pub fn init_probe(self) -> Self {
        // async { //todo:async it!
        self.host_driver_layer.init_probe(); //probe hardware and initialize them
        self.usb_driver_layer.init_probe(); //probe driver modules and load them
                                            // }
                                            // .await;
        self
    }
}

// #[cfg(feature = "arceos")]
// pub mod ax;
