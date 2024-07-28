#![no_std]
#![no_main]
use core::ptr;
use core::time::Duration;
use log::*;
use axhal::time::busy_wait;

pub const FI2C_CONFIG_TBL: [FI2cConfig; FI2C_NUM] = [
    #[cfg(feature = "i2c0")]
    FI2cConfig {
        instance_id: FI2C0_ID,
        base_addr: FI2C0_BASE_ADDR,
        irq_num: FI2C0_IRQ_NUM,
        irq_priority: 0,
        ref_clk_hz: FI2C_CLK_FREQ_HZ,
        work_mode: FI2C_MASTER,
        slave_addr: 0,
        use_7bit_addr: true,
        speed_rate: FI2C_SPEED_STANDARD_RATE,
    },
    #[cfg(feature = "i2c1")]
    FI2cConfig {
        instance_id: 1,
        base_addr: 0x28012000,
        irq_num: 122,
        irq_priority: 0,
        ref_clk_hz: 50000000,
        work_mode: 0,
        slave_addr: 0,
        use_7bit_addr: true,
        speed_rate: 100000,
    },
    #[cfg(feature = "i2c2")]
    FI2cConfig {
        instance_id: FI2C2_ID,
        base_addr: FI2C2_BASE_ADDR,
        irq_num: FI2C2_IRQ_NUM,
        irq_priority: 0,
        ref_clk_hz: FI2C_CLK_FREQ_HZ,
        work_mode: FI2C_MASTER,
        slave_addr: 0,
        use_7bit_addr: true,
        speed_rate: FI2C_SPEED_STANDARD_RATE,
    },
    #[cfg(feature = "i2c3")]
    FI2cConfig {
        instance_id: FI2C3_ID,
        base_addr: FI2C3_BASE_ADDR,
        irq_num: FI2C3_IRQ_NUM,
        irq_priority: 0,
        ref_clk_hz: FI2C_CLK_FREQ_HZ,
        work_mode: FI2C_MASTER,
        slave_addr: 0,
        use_7bit_addr: true,
        speed_rate: FI2C_SPEED_STANDARD_RATE,
    },
];

pub fn fi2c_lookup_config(instance_id: u32) -> Option<&'static FI2cConfig> {
    let mut ptr: Option<&FI2cConfig> = None;

    for index in 0..FI2C_NUM {
        unsafe {
            if FI2C_CONFIG_TBL[index].instance_id == instance_id {
                ptr = Some(&FI2C_CONFIG_TBL[index]);
                break;
            }
        }
    }

    ptr
}


