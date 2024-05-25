use axhal::mem::{self, VirtAddr};
use conquer_once::spin::OnceCell;
use futures_intrusive::buffer;
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;

use super::{
    registers,
    xhci_command_manager::COMMAND_MANAGER,
    xhci_slot_manager::{self, SLOT_MANAGER},
};

pub(crate) static SCRATCH_PAD: OnceCell<Spinlock<ScratchPad>> = OnceCell::uninit();

struct ScratchPad {
    buffer: PageBox<[[usize; mem::PAGE_SIZE_4K]]>,
    buffer_indexs: PageBox<[VirtAddr]>,
}

pub fn new() {
    registers::handle(|r| {
        let max_scratchpad_buffers = r
            .capability
            .hcsparams2
            .read_volatile()
            .max_scratchpad_buffers();
        let mut scratch_pad = ScratchPad {
            buffer: PageBox::alloc_pages(
                max_scratchpad_buffers.try_into().unwrap(),
                [0 as usize; mem::PAGE_SIZE_4K],
            ),
            buffer_indexs: PageBox::new_slice(
                VirtAddr::from(0),
                max_scratchpad_buffers.try_into().unwrap(),
            ),
        };

        unsafe {
            scratch_pad
                .buffer
                .iter()
                .zip(scratch_pad.buffer_indexs.iter_mut())
                .for_each(|(l, r)| {
                    debug!("check this add is not zero? {:x}", l.as_ptr().addr());
                    // (*r) = VirtAddr::from(*l as usize);
                    (*r) = VirtAddr::from(l.as_ptr().addr());
                })
        }

        SCRATCH_PAD.init_once(move || Spinlock::new(scratch_pad));
        assign_scratchpad_into_dcbaa();
    });

    debug!("initialized!");
}

pub fn assign_scratchpad_into_dcbaa() {
    xhci_slot_manager::set_dcbaa(&SCRATCH_PAD.get().unwrap().lock().buffer_indexs); //SUS
                                                                                    //TODO Redundent design, simplify it.
    debug!("initialized!");
}
