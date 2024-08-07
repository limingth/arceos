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
    
}
