use alloc::{sync::Arc, vec::Vec};
use spinlock::SpinNoIrq;

use crate::abstractions::PlatformAbstractions;

use self::driverapi::USBSystemDriverModule;

pub mod driverapi;

pub struct DriverContainers<'a, O>
where
    O: PlatformAbstractions,
{
    drivers: Vec<Arc<SpinNoIrq<dyn USBSystemDriverModule<'a, O>>>>,
}

impl<'a, O> DriverContainers<'a, O>
where
    O: PlatformAbstractions,
{
    pub fn new() -> Self {
        DriverContainers {
            drivers: Vec::new(),
        }
    }

    pub fn load_driver(&mut self, mut module: Arc<SpinNoIrq<dyn USBSystemDriverModule<'a, O>>>) {
        module.lock().preload_module();
        self.drivers.push(module)
    }
}
