#![no_std]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![allow(warnings)]
#![feature(auto_traits)]
#![feature(btreemap_alloc)]
#![feature(if_let_guard)]
#![feature(get_many_mut)]
#![feature(let_chains)]

extern crate alloc;

pub mod abstractions;
pub mod err;
pub mod glue;
pub mod host;
pub mod usb;

// #[cfg(feature = "arceos")]
// pub mod ax;
