use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::context::Device;

use crate::dma::DMAVec;

use super::registers;

pub(crate) static ROOT_HUB: OnceCell<Spinlock<Roothub>> = OnceCell::uninit();

pub struct RootPort {
    index: usize,
    device: Option<Device64Byte>,
}

pub struct Roothub {
    ports: usize,
    root_ports: PageBox<[Option<Arc<Spinlock<RootPort>>>]>,
}

pub(crate) fn new() {
    registers::handle(|r| {
        let number_of_ports = r.capability.hcsparams1.read_volatile().number_of_ports();
        let root_ports = PageBox::new_slice(Option::None, number_of_ports);
        for i in 0..number_of_ports {
            root_ports[i] = Some(Arc::new(Spinlock::new(RootPort {
                index: i,
                device: Option::None,
            })))
        }
        ROOT_HUB.init_once(move || Roothub {
            ports: number_of_ports,
            root_ports,
        })
    });
}
