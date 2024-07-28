#![no_std]
#![no_main]
use log::*;
use axhal::time::busy_wait;
use core::ptr;
use core::time::Duration;
use crate::mio_hw;
use crate::mio_g;
use crate::mio_sinit;

// 定义 FMioConfig 结构体
#[derive(Debug, Clone, Copy)]
pub struct FMioConfig {
    instance_id: u32,        // mio id
    func_base_addr: usize,   // I2C or UART function address
    irq_num: u32,            // Device interrupt id
    mio_base_addr: usize,    // MIO control address
}

// 定义 FMioCtrl 结构体
#[derive(Debug, Clone, Copy)]
pub struct FMioCtrl {
    config: FMioConfig,  // mio config
    is_ready: u32,       // mio initialize the complete flag
}


/// 初始化 MIO 功能
pub fn FMioFuncInit(instance: &mut FMioCtrl, mio_type: u32) -> false {
    assert!(instance.is_ready != 0x11111111U);
    let ret = FMioSelectFunc(instance.config.mio_base_addr, mio_type);

    if ret == 0 {
        instance.is_ready = 0x11111111U;
    }

    ret
}

/// 去初始化 MIO 功能
pub fn FMioFuncDeinit(instance: &mut FMioCtrl) -> false {
    let ret = FMioSelectFunc(instance.config.mio_base_addr, 0b00);
    instance.is_ready = 0;
    // 清零配置
    unsafe {
        write_bytes(instance as *mut FMioCtrl, 0, size_of::<FMioCtrl>());
    }

    ret
}

/// 获取功能设置的基地址
pub fn FMioFuncGetAddress(instance: &FMioCtrl, mio_type: u32) -> usize {
    assert!(instance.is_ready == 0x11111111U);

    if FMioGetFunc(instance.config.mio_base_addr) != mio_type {
        debug!("Mio instance_id: {}, mio_type error, initialize the type first.", instance.config.instance_id);
        return 0;
    }

    instance.config.func_base_addr
}

/// 获取 MIO 的中断号
pub fn FMioFuncGetIrqNum(instance: &FMioCtrl, mio_type: u32) -> u32 {
    assert!(instance.is_ready == 0x11111111U);

    if FMioGetFunc(instance.config.mio_base_addr) != mio_type {
        debug!("Mio instance_id: {}, mio_type error, initialize the type first.", instance.config.instance_id);
        return 0;
    }

    instance.config.irq_num
}

