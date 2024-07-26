pub mod drivers;
pub mod operation;
use crate::abstractions::PlatformAbstractions;

pub mod descriptors;
pub mod trasnfer;
pub mod urb;

pub struct USBDriverSystem;

impl USBDriverSystem {
    pub fn init(&self) {}

    pub fn init_probe(&self) {}
}
