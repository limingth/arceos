#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![allow(warnings)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;


use driver_usb::host::{USBHost, USBHostConfig, xhci::Xhci};
use axalloc::GlobalNoCacheAllocator;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
        let phytium_cfg_id_0 = (0xffff_0000_31a0_8000, 48, 0);


    let config = USBHostConfig::new(
        phytium_cfg_id_0.0, phytium_cfg_id_0.1, phytium_cfg_id_0.2, GlobalNoCacheAllocator::new());
        
    let usb = USBHost::new::<Xhci>(config).unwrap();


}
