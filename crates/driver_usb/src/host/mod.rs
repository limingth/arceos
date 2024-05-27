// A workaround for the `derive_builder` crate.
#![allow(clippy::default_trait_access)]

use self::structures::{extended_capabilities, registers};

pub(crate) mod exchanger;
mod mapper;
mod page_box;
mod port;
mod structures;
mod xhc;

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
