use crate::host::page_box::PageBox;

use super::{dcbaa, registers};
use alloc::vec::Vec;
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use core::alloc::Layout;
use core::convert::TryInto;
use os_units::Bytes;

static SCRATCHPAD: OnceCell<Scratchpad> = OnceCell::uninit();

pub(crate) fn init() {
    if Scratchpad::needed() {
        init_static();
    }
}

fn init_static() {
    let mut scratchpad = Scratchpad::new();
    scratchpad.init();
    scratchpad.register_with_dcbaa();

    SCRATCHPAD.init_once(|| scratchpad)
}

struct Scratchpad {
    arr: PageBox<[VirtAddr]>,
    bufs: Vec<PageBox<[u8]>>,
}
impl Scratchpad {
    fn new() -> Self {
        let len: usize = Self::num_of_buffers().try_into().unwrap();

        Self {
            arr: PageBox::new_slice(VirtAddr::from(0), len),
            bufs: Vec::new(),
        }
    }

    fn needed() -> bool {
        Self::num_of_buffers() > 0
    }

    fn init(&mut self) {
        self.allocate_buffers();
        self.write_buffer_addresses();
    }

    fn register_with_dcbaa(&self) {
        dcbaa::register_device_context_addr(0, self.arr.virt_addr());
    }

    fn allocate_buffers(&mut self) {
        let layout =
            Layout::from_size_align(Self::page_size().as_usize(), Self::page_size().as_usize());
        let layout = layout.unwrap_or_else(|_| {
            panic!(
                "Failed to create a layout for {} bytes with {} bytes alignment",
                Self::page_size().as_usize(),
                Self::page_size().as_usize()
            )
        });

        for _ in 0..Self::num_of_buffers() {
            let b = PageBox::from_layout_zeroed(layout);

            self.bufs.push(b);
        }
    }

    fn write_buffer_addresses(&mut self) {
        let page_size: usize = Self::page_size().as_usize();
        for (x, buf) in self.arr.iter_mut().zip(self.bufs.iter()) {
            *x = buf.virt_addr().align_up(page_size);
        }
    }

    fn num_of_buffers() -> u32 {
        registers::handle(|r| {
            r.capability
                .hcsparams2
                .read_volatile()
                .max_scratchpad_buffers()
        })
    }

    fn page_size() -> Bytes {
        Bytes::new(registers::handle(|r| r.operational.pagesize.read_volatile().get()).into())
    }
}
