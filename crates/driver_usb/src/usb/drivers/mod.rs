use alloc::{sync::Arc, vec::Vec};
use spinlock::SpinNoIrq;

use self::driverapi::USBSystemDriverModule;

pub mod driverapi;

pub struct DriverContainers<'a> {
    drivers: Vec<Arc<SpinNoIrq<dyn USBSystemDriverModule<'a>>>>,
}

impl<'a> DriverContainers<'a> {
    pub fn new() -> Self {
        DriverContainers {
            drivers: Vec::new(),
        }
    }

    pub fn load_driver(&mut self, mut module: Arc<SpinNoIrq<dyn USBSystemDriverModule<'a>>>) {
        module.lock().preload_module();
        self.drivers.push(module)
    }
}
