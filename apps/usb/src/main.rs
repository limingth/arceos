#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![allow(warnings)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

use axalloc::GlobalNoCacheAllocator;
use driver_usb::{
    host::{xhci::Xhci, USBHost, USBHostConfig},
    OsDep,
};

#[derive(Clone)]
struct OsDepImp;

impl OsDep for OsDepImp {
    type DMA = GlobalNoCacheAllocator;

    const PAGE_SIZE: usize = axalloc::PAGE_SIZE;
    fn dma_alloc(&self) -> Self::DMA {
        axalloc::global_no_cache_allocator()
    }
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let phytium_cfg_id_0 = (0xffff_0000_31a0_8000, 48, 0);

    let config = USBHostConfig::new(
        phytium_cfg_id_0.0,
        phytium_cfg_id_0.1,
        phytium_cfg_id_0.2,
        OsDepImp {},
    );

    let usb = USBHost::new::<Xhci<_>>(config).unwrap();

    usb.poll();
}
