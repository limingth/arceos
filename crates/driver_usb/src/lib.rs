//! Common traits and types for graphics display device drivers.

#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]
#![feature(auto_traits)]
#![feature(btreemap_alloc)]
#![feature(if_let_guard)]
#![feature(get_many_mut)]
#![feature(let_chains)]

use core::alloc::Allocator;

extern crate alloc;
pub(crate) mod addr;
pub(crate) mod device_types;
pub(crate) mod dma;
pub mod err;
pub mod host;
pub mod platform_spec;

#[cfg(feature = "arceos")]
pub mod ax;

pub trait OsDep: Clone + Send + Sync + Sized {
    const PAGE_SIZE: usize;
    type DMA: Allocator + Send + Sync + Clone;
    fn dma_alloc(&self) -> Self::DMA;
    fn force_sync_cache();
}
