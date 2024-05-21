//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]

extern crate alloc;
pub(crate) mod dma;
pub(crate) mod addr;
pub(crate) mod device_types;
pub mod host;
pub mod err;

#[cfg(feature="arceos")]
pub mod ax;




