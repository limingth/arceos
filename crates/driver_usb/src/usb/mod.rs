pub mod drivers;
pub mod operation;
use alloc::sync::Arc;
use spinlock::SpinNoIrq;

use crate::{abstractions::PlatformAbstractions, USBSystemConfig};

use self::drivers::DriverContainers;

pub mod descriptors;
pub mod trasnfer;
pub(crate) mod universal_drivers;
pub mod urb;

pub struct USBDriverSystem<'a, O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    managed_modules: DriverContainers<'a, O>,
}

impl<'a, O> USBDriverSystem<'a, O>
where
    O: PlatformAbstractions,
{
    pub fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self {
        Self {
            config,
            managed_modules: DriverContainers::new(),
        }
    }

    pub fn init(&self) {}

    pub fn init_probe(&self) {}
}
