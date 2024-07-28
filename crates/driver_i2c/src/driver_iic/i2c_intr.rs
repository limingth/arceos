#![no_std]
#![no_main]
use log::*;
use axhal::time::busy_wait;
use core::time::Duration;

//暂时不用中断，先不翻译完，巨多（悲）










// I2C主机模式下的中断处理函数
pub fn FI2cMasterIntrHandler(vector: i32, param: *mut FI2c) {
    let instance_p = unsafe { &mut *param };
    let base_addr = instance_p.config.base_addr;
    let last_err: u32;
    let stat:u32 = FI2cClearIntrBits(base_addr, &last_err);
    let raw_stat:u32 = input_32(base_addr,0x34);
    let enabled:u32 = input_32(base_addr,0x6C);
    let mut val: u32 = 0;

    assert!(instance_p.config.work_mode == 0);

    if !(enabled & (0x1 << 0) != 0) || !(raw_stat & !(0x1 << 8) != 0) {
        return;
    }

    if stat & (0x1 << 6) != 0 {
        debug!("last error: 0x{:x}", last_err);
        deubg!("abort source: 0x{:x}", input_32(base_addr,0x80));
        instance_p.status = 0x3;
        output_32(base_addr,0x30,0); // Disable all interrupts
        input_32(base_addr,0x54); // Clear abort
        output_32(base_addr,0x6C,1); // Re-enable I2C
        FI2cMasterCallEvtHandler(instance_p, 0, &mut val);
        return;
    }

    if stat & (0x1 << 2) != 0 {
        FI2cMasterIntrRxFullHandler(instance_p);
        FI2cMasterCallEvtHandler(instance_p, 1, &mut val);
        return;
    }

    if stat & (0x1 << 4) != 0 {
        FI2cMasterIntrTxEmptyHandler(instance_p);
        FI2cMasterCallEvtHandler(instance_p, 2, &mut val);
        return;
    }
}

// I2C从机模式下的中断处理函数
pub fn FI2cSlaveIntrHandler(vector: i32, param: *mut FI2c) {
    let instance_p = unsafe { &mut *param };
    let base_addr = instance_p.config.base_addr;
    let mut last_err: u32;

    let stat = input_32(base_addr,0x2C);
    let raw_stat = input_32(base_addr,0x34);
    let enabled = input_32(base_addr,0x6C);
    let slave_active = (input_32(base_addr,0x70) & (0x1 << 6)) != 0;
    let mut val: u8 = 0;
    let mut reg_val: u32;

    assert!(instance_p.config.work_mode == 1);

    if !(enabled & (0x1 << 0) != 0) || !(raw_stat & !(0x1 << 8) != 0) {
        return;
    }

    let stat = FI2cClearIntrBits(base_addr, &mut last_err);

    if stat & (0x1 << 2) != 0 {
        if instance_p.status != 0x1 {
            instance_p.status = 0x1;
            FI2cSlaveCallEvtHandler(instance_p, 1, &mut val);
        }
        val = input_32(base_addr,0x10) as u8;
        FI2cSlaveCallEvtHandler(instance_p, 3, &mut val);
    }

    if stat & (0x1 << 5) != 0 {
        if slave_active {
            input_32(base_addr,0x50); // Clear read request
            instance_p.status = 0x1;
            FI2cSlaveCallEvtHandler(instance_p, 0, &mut val);
            reg_val = val as u32;
            output_32(base_addr,0x10,reg_val);
        }
    }

    if stat & (0x1 << 7)  != 0 {
        FI2cSlaveCallEvtHandler(instance_p, 2, &mut val);
        input_32(base_addr,0x58); // Clear RX done
        return;
    }

    if stat & (0x1 << 9) != 0 {
        instance_p.status = 0x0;
        FI2cSlaveCallEvtHandler(instance_p, 4, &mut val);
    }

    if stat & (0x1 << 6) != 0 {
        instance_p.status = 0x3;
        fi2c_slave_call_evt_handler(instance_p, 5, &mut val);
        fi2c_error!("last error: 0x{:x}", last_err);
        fi2c_error!("abort source: 0x{:x}", input_32(base_addr,0x80));
    }
}
