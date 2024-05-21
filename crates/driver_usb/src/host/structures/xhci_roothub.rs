use core::mem::MaybeUninit;
use core::{num, option, panic, result};

use alloc::string::String;
use alloc::sync::Arc;
use axalloc::GlobalNoCacheAllocator;
use conquer_once::spin::OnceCell;
use log::{debug, error, info};
use page_box::PageBox;
use spinning_top::{lock_api::Mutex, Spinlock};
use xhci::context::{Device64Byte, DeviceHandler};
use xhci::{context::Device, registers::PortRegisterSet};

use crate::host::structures::xhci_command_manager::{CommandResult, COMMAND_MANAGER};
use crate::host::structures::xhci_slot_manager::{SlotManager, SLOT_MANAGER};
use crate::{dma::DMAVec, host::structures::XHCI_CONFIG_MAX_PORTS};

use super::xhci_usb_device::XHCIUSBDevice;
use super::{registers, USBSpeed};

// 定义静态变量ROOT_HUB，用于存储根集线器的实例
pub(crate) static ROOT_HUB: OnceCell<Spinlock<Roothub>> = OnceCell::uninit();

pub struct RootPort {
    root_port_id: usize,
    device: Arc<MaybeUninit<XHCIUSBDevice>>,
    device_inited: bool,
}

impl RootPort {
    pub fn configure(&mut self) {}

    pub fn initialize(&mut self) {
        if !self.connected() {
            error!("port {} not connected", self.root_port_id);
            return;
        }
        debug!("port {} connected, continue", self.root_port_id);

        registers::handle(|r| {
            // r.port_register_set.read_volatile_at(self.index).portsc.port_link_state() // usb 3, not complete code
            //DEBUG lets just use usb 2 job sequence? should be compaible? might stuck at here
            r.port_register_set
                .update_volatile_at(self.root_port_id, |prs| {
                    prs.portsc.set_port_reset();

                    prs.portsc.set_0_port_enabled_disabled();

                    debug!("waiting for port reset!");
                    while !prs.portsc.port_reset() {}
                })
        });

        // //waiting for reset
        // while !registers::handle(|r| {
        //     r.port_register_set
        //         .read_volatile_at(self.root_port_id)
        //         .portsc
        //         .port_reset_change()
        // }) {}

        debug!("port {} reset!", self.root_port_id);

        let get_speed = self.get_speed();
        if get_speed == USBSpeed::USBSpeedUnknown {
            error!("unknown speed, index:{}", self.root_port_id);
        }
        debug!("port speed: {:?}", get_speed);

        debug!("initializing device: {:?}", get_speed);

        if let Ok(device) = XHCIUSBDevice::new(self.root_port_id as u8) {
            debug!("writing ...");
            self.device_inited = true;
            unsafe {
                Arc::get_mut(&mut self.device) //TODO assert device allocated
                    .unwrap()
                    .write(device)
                    .initialize()
            };
            debug!("writing complete");
        }
        debug!("initialize complete");
    }

    pub fn status_changed(&self) {
        // 检查MMIO（内存映射I/O），确保索引在有效范围内
        assert!(self.root_port_id < XHCI_CONFIG_MAX_PORTS);
        registers::handle(|r| {
            r.port_register_set
                .update_volatile_at(self.root_port_id, |port_register_set| {
                    // TODO: check here
                    port_register_set.portsc.clear_port_enabled_disabled();
                });
            // TODO: is plug and play support
            if self.device_inited
            /* and if is plug and play? assume is! */
            && r.port_register_set.read_volatile_at(self.root_port_id).portsc.current_connect_status()
            {
                unsafe { self.device.assume_init_mut().status_changed() };
            }
        })
    }

    fn get_speed(&self) -> USBSpeed {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.root_port_id)
                .portsc
                .port_speed()
        })
        .into()
    }

    pub fn connected(&self) -> bool {
        registers::handle(|r| {
            r.port_register_set
                .read_volatile_at(self.root_port_id)
                .portsc
                .current_connect_status()
        })
    }
}

pub struct Roothub {
    ports: usize,
    root_ports: PageBox<[Arc<MaybeUninit<Spinlock<RootPort>>>]>,
}

impl Roothub {
    pub fn initialize(&mut self) {
        //todo delay?
        debug!("initializing root ports");
        self.root_ports
            .iter_mut()
            .map(|a| unsafe { a.clone().assume_init() })
            .for_each(|arc| {
                arc.lock().initialize();
            });

        debug!("configuring root ports");
        self.root_ports
            .iter_mut()
            .map(|a| unsafe { a.clone().assume_init() })
            .for_each(|arc| {
                arc.lock().configure();
            });
    }
}

// 当接收到根端口状态变化的通知时调用
// 这里似乎产生了无限循环
pub(crate) fn status_changed(uch_port_id: u8) {
    // 将UCH端口ID转换为索引，并确保索引在有效范围内
    let n_port = uch_port_id as usize - 1;
    debug!("try to lock!,port:{}", n_port);
    let mut root_hub = ROOT_HUB
        .try_get()
        .expect("ROOT_HUB is not initialized")
        .data_ptr();
    debug!("locked!");
    assert!(
        n_port < unsafe { (*root_hub).ports },
        "Port index out of bounds"
    );

    // 如果端口存在，则更新其状态
    //丑陋，临时解决策略
    unsafe { (*(*(*root_hub).root_ports[n_port].as_ptr()).data_ptr()).status_changed() }
    // if let arc_root_port =
    //     unsafe { (*(*(*root_hub).root_ports[n_port].as_mut_ptr()).data_ptr()).status_changed() }
    // {
    //     //check: does clone affect value?
    //     let mut root_port = unsafe { arc_root_port.status_changed() };
    // } else {
    //     panic!("Root port doesn't exist");
    // }
}

pub(crate) fn new() {
    // 通过MMIO读取根集线器支持的端口数量
    registers::handle(|r| {
        let number_of_ports = r.capability.hcsparams1.read_volatile().number_of_ports() as usize;
        let mut root_ports = PageBox::new_slice(Arc::new_uninit(), number_of_ports); //DEBUG: using nocache allocator
        debug!("number of ports:{}", number_of_ports);
        root_ports
            .iter_mut()
            .enumerate()
            .for_each(|(i, port_uninit)| {
                debug!("allocating port {i}");
                unsafe { Arc::get_mut_unchecked(port_uninit) }.write(Spinlock::new(RootPort {
                    root_port_id: i,
                    device: Arc::new_uninit(),
                    device_inited: false,
                }));
                debug!("assert:{} == {i}", unsafe {
                    port_uninit.clone().assume_init().lock().root_port_id
                })
            });
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
