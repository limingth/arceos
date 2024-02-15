use core::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    clone,
};

use alloc::vec::{self, Vec};
use axhal::mem::PhysAddr;
use xhci::Registers;

use super::dcbaa::DeviceContextBaseAddressArray;

pub static mut SCRATCHPAD: Option<RefCell<Scratchpad>> = None;

pub fn init_once(r: &Ref<Registers>, dcbaa: &mut DeviceContextBaseAddressArray) {
    let mut scratchpad = Scratchpad::new(r);
    scratchpad.init();
    scratchpad.register_with_dcbaa(dcbaa);

    unsafe { SCRATCHPAD = Some(RefCell::new(scratchpad)) }
}

struct Scratchpad {
    arr: Vec<PhysAddr>,
    bufs: Vec<Vec<u8>>,
    page_size: Fn() -> usize,
    buffer_size: Fn() -> usize,
}
impl Scratchpad {
    fn new(r: &Ref<Registers>) -> Self {
        let buffer_size = {
            let r_1 = r.borrow();
            move || {
                r_1.capability
                    .hcsparams2
                    .read_volatile()
                    .max_scratchpad_buffers() as usize
            }
        };

        let page_size = {
            let r_2 = r.borrow();
            move || r_2.operational.pagesize.read_volatile().get() as usize
        };

        page_size = Self {
            arr: vec![PhysAddr::zero(); buffer_size()],
            bufs: Vec::new(),
            page_size: page_size,
            buffer_size: buffer_size,
        }
    }

    fn exists(&self) -> bool {
        self.num_of_buffers() > 0
    }

    fn init(&mut self) {
        self.allocate_buffers();
        self.write_buffer_addresses();
    }

    fn register_with_dcbaa(&self, dcbaa: &mut DeviceContextBaseAddressArray) {
        dcbaa[0] = self.arr.phys_addr();
    }

    fn allocate_buffers(&mut self) {
        for _ in 0..self.buffer_size() {
            // Allocate the double size of memory, then register the aligned address with the
            // array.
            let b = vec![0, Self.page_size() * 2];
            self.bufs.push(b);
        }
    }

    fn write_buffer_addresses(&mut self) {
        let page_size: u64 = self.page_size().into();
        for (x, buf) in self.arr.iter_mut().zip(self.bufs.iter()) {
            *x = buf.phys_addr().align_up(page_size);
        }
    }

    fn num_of_buffers(&self) -> usize {
        self.buffer_size()
    }

    fn page_size(&self) -> usize {
        self.page_size()
    }
}
