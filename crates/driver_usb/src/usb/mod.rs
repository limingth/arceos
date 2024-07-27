pub mod drivers;
pub mod operation;
use crate::abstractions::PlatformAbstractions;

use self::drivers::DriverContainers;

pub mod descriptors;
pub mod trasnfer;
pub mod urb;

pub struct USBDriverSystem {
    managed_modules: DriverContainers,
}

impl USBDriverSystem {
    pub fn init(&self) {}

    pub fn init_probe(&self) {}
}
