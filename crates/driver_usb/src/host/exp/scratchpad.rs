use core::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell},
    clone,
};

use alloc::vec::Vec;
use alloc::{boxed::Box, vec};
use axhal::mem::PhysAddr;
use xhci::{accessor::Mapper, Registers};

use super::dcbaa::{DeviceContextBaseAddressArray, DCBAA};

pub static mut SCRATCHPAD: Option<RefCell<Scratchpad>> = None;

pub fn init_once(r: &'static Registers<impl Mapper + Clone>) {
    let mut scratchpad = Scratchpad::new(r);
    scratchpad.init();
    scratchpad.register_with_dcbaa();

    unsafe { SCRATCHPAD = Some(RefCell::new(scratchpad)) }
}

struct Scratchpad {
    arr: Vec<PhysAddr>,
    bufs: Vec<Vec<u8>>,
    page_size: Vec<dyn Fn() -> usize>,
    buffer_size: Vec<dyn Fn() -> usize>,
}
impl Scratchpad {
    fn new(r: &'static Registers<impl Mapper + Clone>) -> Self {
        let buffer_size = Vec::new({
            let r_1 = r.borrow();
            move || {
                r_1.capability
                    .hcsparams2
                    .read_volatile()
                    .max_scratchpad_buffers() as usize
            }
        });

        let page_size = Vec::new({
            let r_2 = r.borrow();
            move || r_2.operational.pagesize.read_volatile().get() as usize
        });

        Self {
            arr: vec![PhysAddr::from(0); buffer_size()],
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

    fn register_with_dcbaa(&mut self) {
        if let Some(ref mut dcbaa) = unsafe { DCBAA.borrow_mut() } {
            dcbaa.borrow_mut().lock()[0] = self.arr.as_mut_ptr().addr().into();
        }
    }

    fn allocate_buffers(&mut self) {
        for _ in 0..(self.buffer_size)() {
            // Allocate the double size of memory, then register the aligned address with the
            // array.
            let b = vec![0 as u8; (self.page_size)() * 2];
            self.bufs.push(b);
        }
    }

    fn write_buffer_addresses(&mut self) {
        let page_size = (self.page_size)();
        for (x, buf) in self.arr.iter_mut().zip(self.bufs.iter()) {
            *x = (PhysAddr::from(buf.as_ptr().addr())).align_up(page_size);
        }
    }

    fn num_of_buffers(&self) -> usize {
        (self.buffer_size)()
    }

    fn page_size(&self) -> usize {
        self.page_size()
    }
}
