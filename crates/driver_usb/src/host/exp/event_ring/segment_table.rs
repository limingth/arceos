// SPDX-License-Identifier: GPL-3.0-or-later
//copy from ramen, should rewrite soon

use core::array;

use alloc::vec;
use alloc::{boxed::Box, vec::Vec};
use axhal::mem::PhysAddr;
use log::info;

pub type SegmentTable = Vec<Entry>;

pub fn new(len: usize) -> Vec<SegmentTable> {
    info!("new");
    Vec::new(vec![Entry::null(); len])
}

//TODO rewrite beacuse gpl3
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Entry {
    base_address: u64,
    segment_size: u64,
}
impl Entry {
    // Although the size of segment_size is u64, bits 16:63 are reserved.
    pub fn set(&mut self, addr: PhysAddr, size: u16) {
        info!("set");
        self.base_address = addr.as_usize() as u64;
        self.segment_size = size.into();
    }

    fn null() -> Self {
        info!("null");
        Self {
            base_address: 0,
            segment_size: 0,
        }
    }
}
