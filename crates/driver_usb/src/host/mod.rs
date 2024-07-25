use alloc::{boxed::Box, sync::Arc};
use data_structures::host_controllers::{xhci::XHCIRegisters, Controller, ControllerArc};
use spinlock::SpinNoIrq;

use crate::{abstractions::PlatformAbstractions, USBSystemConfig};

pub mod data_structures;

impl<O> USBSystemConfig<O>
where
    O: PlatformAbstractions,
{
    pub fn new(mmio_base_addr: usize, irq_num: u32, irq_priority: u32, os_dep: O) -> Self {
        let base_addr = O::VirtAddr::from(mmio_base_addr);
        Self {
            base_addr,
            irq_num,
            irq_priority,
            os: os_dep,
        }
    }
}

#[derive(Clone)]
pub struct USBHostSystem<O>
where
    O: PlatformAbstractions,
{
    pub(crate) config: USBSystemConfig<O>,
    pub(crate) controller: ControllerArc<O>,
}

impl<O> USBHostSystem<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: USBSystemConfig<O>) -> crate::err::Result<Self> {
        let controller = Arc::new(SpinNoIrq::new({
            let xhciregisters: Box<(dyn Controller<O> + 'static)> = {
                if cfg!(xhci) {
                    Box::new(XHCIRegisters::new(config.clone()))
                } else {
                    panic!("no host controller defined")
                }
            };
            xhciregisters
        }));
        Ok(Self { config, controller })
    }

    pub fn init(&self) {
        self.controller.lock().init()
    }

    pub fn init_probe(&self) {
        self.controller.lock().probe()
    }

    // pub fn poll(&self) -> crate::err::Result {
    //     let controller = self.controller.clone();
    //     let mut g = self.controller.lock();
    //     let mut device_list = g.poll(controller)?;

    //     let mut dl = self.device_list.lock();
    //     dl.append(&mut device_list);
    //     Ok(())
    // }
}
