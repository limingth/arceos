#![no_std]
#![no_main]
#![allow(warnings)]

#[macro_use]
extern crate axstd as std;

use core::time::Duration;
use std::thread::sleep;

use axalloc::GlobalNoCacheAllocator;
use driver_usb::{
    host::{
        xhci::{self, Xhci},
        USBHost, USBHostConfig,
    },
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

    fn force_sync_cache() {}
}

#[no_mangle]
fn main() {
    let phytium_cfg_id_0 = (0xffff_0000_31a0_8000, 48, 0);

    let config = USBHostConfig::new(
        phytium_cfg_id_0.0,
        phytium_cfg_id_0.1,
        phytium_cfg_id_0.2,
        OsDepImp {},
    );

    let mut usb = USBHost::new::<Xhci<_>>(config).unwrap();

    sleep(Duration::from_millis(300));

    usb.poll().unwrap();

    let mut device_list = usb.device_list();

    let hid = device_list.pop().unwrap();

    hid.test_hid().unwrap();
    // hid.test_mass_storage().unwrap();
}
