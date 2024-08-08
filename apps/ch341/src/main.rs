#![no_std]
#![no_main]
#![allow(warnings)]

use axalloc::GlobalNoCacheAllocator;
use axhal::{mem::VirtAddr, paging::PageSize};
use driver_usb::{USBSystem, USBSystemConfig};

#[macro_use]
extern crate axstd as std;






#[no_mangle]
fn main() {
    let mut usbsystem = driver_usb::USBSystem::new({
        USBSystemConfig::new(0xffff_0000_31a0_8000, 48, 0, PlatformAbstraction)
    })
    .init()
    .init_probe()
    .drive_all();
}
