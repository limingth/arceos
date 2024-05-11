use core::{option, panic};

use aarch64_cpu::registers::VTCR_EL2::SH0::Non;
use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use log::debug;
use page_box::PageBox;
use spinning_top::{lock_api::Mutex, Spinlock};
use xhci::context::Device64Byte;
use xhci::{context::Device, registers::PortRegisterSet};

use crate::{dma::DMAVec, host::structures::XHCI_CONFIG_MAX_PORTS};

use super::registers;

// 定义静态变量ROOT_HUB，用于存储根集线器的实例
pub(crate) static ROOT_HUB: OnceCell<Spinlock<Roothub>> = OnceCell::uninit();

pub struct RootPort {
    index: usize,
    device: Option<Device64Byte>,
}

pub struct Roothub {
    ports: usize,
    root_ports: PageBox<[Option<Arc<Spinlock<RootPort>>>]>,
}

impl RootPort {
    pub fn status_changed(&self) {
        // 检查MMIO（内存映射I/O），确保索引在有效范围内
        assert!(self.index < XHCI_CONFIG_MAX_PORTS);
        registers::handle(|r| {
            r.port_register_set
                .update_volatile_at(self.index, |port_register_set| {
                    // TODO: check here
                    port_register_set.portsc.clear_port_enabled_disabled();
                })
            // TODO: is plug and play support
        })
    }
}

// 当接收到根端口状态变化的通知时调用
pub(crate) fn status_changed(uch_port_id: u8) {
    // 将UCH端口ID转换为索引，并确保索引在有效范围内
    let n_port = uch_port_id as usize - 1;
    let mut root_hub = ROOT_HUB
        .try_get()
        .expect("ROOT_HUB is not initialized")
        .lock();
    assert!(n_port < root_hub.ports, "Port index out of bounds");

    // 如果端口存在，则更新其状态
    if let Some(arc_root_port) = &root_hub.root_ports[n_port] {
        let mut root_port = arc_root_port.lock();
        root_port.status_changed();
    } else {
        panic!("Root port doesn't exist");
    }
}

pub(crate) fn new() {
    // 通过MMIO读取根集线器支持的端口数量
    registers::handle(|r| {
        let number_of_ports = r.capability.hcsparams1.read_volatile().number_of_ports() as usize;
        let mut root_ports = PageBox::new_slice(Option::None, number_of_ports);
        for i in 0..number_of_ports {
            root_ports[i] = Some(Arc::new(Spinlock::new(RootPort {
                index: i as usize,
                device: Option::None,
            })))
        }
        // 初始化ROOT_HUB静态变量
        ROOT_HUB.init_once(move || {
            Roothub {
                ports: number_of_ports as usize,
                root_ports,
            }
            .into()
        })
    });

    debug!("initialized!");
}
