use alloc::{sync::Arc, vec::Vec};
use spinlock::SpinNoIrq;

use self::driverapi::USBSubSystemDriverModule;

pub mod driverapi;

pub struct DriverContainers {
    drivers: Vec<Arc<SpinNoIrq<USBSubSystemDriverModule>>>,
}

impl DriverContainers {
    pub fn new() -> Self {
        DriverContainers {
            drivers: Vec::new(),
        }
    }

    pub fn load_driver(&mut self, module: USBSubSystemDriverModule) {

        // self.drivers.push()
    }
}
