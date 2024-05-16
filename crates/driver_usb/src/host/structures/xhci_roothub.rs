use core::{num, option, panic, result};

use aarch64_cpu::registers::VTCR_EL2::SH0::Non;
use alloc::string::String;
use alloc::sync::Arc;
use conquer_once::spin::OnceCell;
use log::{debug, error, info};
use page_box::PageBox;
use spinning_top::{lock_api::Mutex, Spinlock};
use xhci::context::{Device64Byte, DeviceHandler};
use xhci::{context::Device, registers::PortRegisterSet};

use crate::host::structures::xhci_command_manager::{CommandResult, COMMAND_MANAGER};
use crate::host::structures::xhci_slot_manager::{SlotManager, SLOT_MANAGER};
use crate::{dma::DMAVec, host::structures::XHCI_CONFIG_MAX_PORTS};

use super::{registers, USBSpeed};

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

    pub fn initialize(&mut self) -> Result<(), &str> {
        if !self.connected() {
            return Err("not connected");
        }

        registers::handle(|r| {
            // r.port_register_set.read_volatile_at(self.index).portsc.port_link_state() // usb 3, not complete code
            //lets just use usb 2 job sequence? should be compaible
            r.port_register_set.update_volatile_at(self.index, |prs| {
                prs.portsc.port_reset();

                prs.portsc.set_port_reset();

                prs.portsc.set_0_port_enabled_disabled();

                debug!("waiting for port reset!");
                while !prs.portsc.port_reset() {}
            })
        });

        let get_speed = self.get_speed();
        if get_speed == USBSpeed::USBSpeedUnknown {
            error!("unknown speed, index:{}", self.index);
            return Err("unknow index");
        }
        info!("port speed: {:?}", get_speed);

        let mut device = Device::new_64byte();

        debug!("initializing device 64!");

        if let Some(manager) = COMMAND_MANAGER.get() {
            match manager.lock().enable_slot() {
                CommandResult::Success(code, Some(asserted_slot_id)) => {
                    SLOT_MANAGER
                        .get()
                        .unwrap()
                        .lock()
                        .assign_device(asserted_slot_id, device);

                    {}
                }
                //需要让device分配在指定的内存空间中
                _ => {
                    error!("failed to enable slot!");
                    return Err("error on enable slot");
                }
            }
        }
        Ok(())
    }

    fn get_speed(&self) -> USBSpeed {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.index)
                .portsc
                .port_speed()
        })
        .into()
    }

    pub fn connected(&self) -> bool {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.index)
                .portsc
                .current_connect_status()
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
        debug!("number of ports:{}", number_of_ports);
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

    //wait 300ms

    //ininialize root port
}
