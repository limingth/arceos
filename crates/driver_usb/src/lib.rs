//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]

extern crate alloc;
pub(crate) mod dma;
pub mod host;
use core::alloc::Allocator;

#[doc(no_inline)]
pub use driver_common::{BaseDriverOps, DevError, DevResult, DeviceType};
use futures_intrusive::sync::{GenericMutex, GenericMutexGuard};
use log::info;
use spinning_top::RawSpinlock;

pub(crate) type Futurelock<T> = GenericMutex<RawSpinlock, T>;
pub(crate) type FuturelockGuard<'a, T> = GenericMutexGuard<'a, RawSpinlock, T>;
