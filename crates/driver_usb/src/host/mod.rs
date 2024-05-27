// A workaround for the `derive_builder` crate.
#![allow(clippy::default_trait_access)]

use core::alloc::Allocator;

use driver_common::{BaseDriverOps, DeviceType};

use self::structures::{extended_capabilities, registers};

pub(crate) mod exchanger;
mod mapper;
mod page_box;
mod port;
mod structures;
mod xhc;

pub struct VL805<A: Allocator + Clone> {
    alloc: A,
    // regs: Registers<MemoryMapper>,
    // extended_capabilities: Option<extended_capabilities::List<MemoryMapper>>,
    base_addr: usize,
}

impl<A: Allocator + Clone + Sync + Send> BaseDriverOps for VL805<A> {
    fn device_name(&self) -> &str {
        "VL805 4-Port USB 3.0 Host Controller"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::USBHost
    }
}

pub fn init_statics(base_addr: usize) {
    // SAFETY: BAR 0 address is passed.
    unsafe {
        registers::init(base_addr.into());
        extended_capabilities::init(base_addr.into());
    }
}

pub fn init_xhci() {
    xhc::init();
}

pub fn enum_port() {
    port::enum_all_connected_port();

    // multitask::add(Task::new_poll(event::task()));
}
