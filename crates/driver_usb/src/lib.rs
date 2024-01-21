//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(strict_provenance)]
pub mod host;


#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use log::info;



