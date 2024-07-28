#![no_std]
#![no_main]
use core::ptr;
use core::time::Duration;
use core::ptr::write_volatile;
use log::*;
use axhal::time::busy_wait;

const FI2C_CON_OFFSET 0x00
const FI2C_TAR_OFFSET 0x04
const FI2C_SAR_OFFSET 0x08
const FI2C_HS_MADDR_OFFSET 0x0C
const FI2C_DATA_CMD_OFFSET 0x10
const FI2C_SS_SCL_HCNT_OFFSET 0x14
const FI2C_SS_SCL_LCNT_OFFSET 0x18
const FI2C_FS_SCL_HCNT_OFFSET 0x1C
const FI2C_FS_SCL_LCNT_OFFSET 0x20
const FI2C_HS_SCL_HCNT_OFFSET 0x24
const FI2C_HS_SCL_LCNT_OFFSET 0x28
const FI2C_INTR_STAT_OFFSET 0x2C
const FI2C_INTR_MASK_OFFSET 0x30
const FI2C_RAW_INTR_STAT_OFFSET 0x34
const FI2C_RX_TL_OFFSET 0x38
const FI2C_TX_TL_OFFSET 0x3C
const FI2C_CLR_INTR_OFFSET 0x40
const FI2C_CLR_RX_UNDER_OFFSET 0x44
const FI2C_CLR_RX_OVER_OFFSET 0x48
const FI2C_CLR_TX_OVER_OFFSET 0x4C
const FI2C_CLR_RD_REQ_OFFSET 0x50
const FI2C_CLR_TX_ABRT_OFFSET 0x54
const FI2C_CLR_RX_DONE_OFFSET 0x58
const FI2C_CLR_ACTIVITY_OFFSET 0x5c
const FI2C_CLR_STOP_DET_OFFSET 0x60
const FI2C_CLR_START_DET_OFFSET 0x64
const FI2C_CLR_GEN_CALL_OFFSET 0x68
const FI2C_ENABLE_OFFSET 0x6C
const FI2C_STATUS_OFFSET 0x70
const FI2C_TXFLR_OFFSET 0x74
const FI2C_RXFLR_OFFSET 0x78
const FI2C_SDA_HOLD_OFFSET 0x7c
const FI2C_TX_ABRT_SOURCE_OFFSET 0x80
const FI2C_SLV_DATA_NACK_ONLY_OFFSET 0x84
const FI2C_DMA_CR_OFFSET 0x88
const FI2C_DMA_TDLR_OFFSET 0x8c
const FI2C_DMA_RDLR_OFFSET 0x90
const FI2C_SDA_SETUP_OFFSET 0x94
const FI2C_ACK_GENERAL_CALL_OFFSET 0x98
const FI2C_ENABLE_STATUS_OFFSET 0x9C
const FI2C_FS_SPKLEN_OFFSET 0xa0
const FI2C_HS_SPKLEN_OFFSET 0xa4
const FI2C_COMP_PARAM_1_OFFSET 0xf4
const FI2C_COMP_VERSION_OFFSET 0xf8
const FI2C_COMP_TYPE_OFFSET 0xfc

const DIV_ROUND_UP(n, d) (((n) + (d)-1) / (d))
const FI2C_IC_SAR_MASK (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (9))))
const FI2C_IC_TAR_MASK (((!0u32) - (1u32 << (0)) + 1) & (!0u32 >> (32 - 1 - (9))))
const FI2C_CON_SPEED_MASK (((!0u32) - (1u32 << 1) + 1) & (!0u32 >> (32 - 1 - 2)));
const FI2C_TIMEOUT: u32 = 50000;

// 定义速度配置相关的结构体
#[derive(Debug, Clone, Copy)]
pub struct FI2cSpeedCfg {
    speed_mode: u32,
    scl_lcnt: u32,
    scl_hcnt: u32,
    sda_hold: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FI2cSpeedModeInfo {
    speed: u32,
    min_scl_hightime_ns: u32,
    min_scl_lowtime_ns: u32,
    def_risetime_ns: u32,
    def_falltime_ns: u32,
}

pub const I2C_SPEED_CFG: [FI2cSpeedModeInfo; 3] = [
    FI2cSpeedModeInfo {
        speed: FI2C_SPEED_STANDARD_RATE,
        min_scl_hightime_ns: 4000,
        min_scl_lowtime_ns: 4700,
        def_risetime_ns: 1000,
        def_falltime_ns: 300,
    },
    FI2cSpeedModeInfo {
        speed: FI2C_SPEED_FAST_RATE,
        min_scl_hightime_ns: 600,
        min_scl_lowtime_ns: 1300,
        def_risetime_ns: 300,
        def_falltime_ns: 300,
    },
    FI2cSpeedModeInfo {
        speed: FI2C_SPEED_HIGH_RATE,
        min_scl_hightime_ns: 390,
        min_scl_lowtime_ns: 460,
        def_risetime_ns: 60,
        def_falltime_ns: 160,
    },
];

//设置I2C控制器的使能状态
pub fn i2c_setenable(addr:u32,enable:bool) -> bool{
    let status:u32 = match enable{
        true => 0x1 << 0,
        false => 0x0 << 0,
    };
    let timeout:u32 = FI2C_TIMEOUT;
    while(0!=timeout--){
        output_32(addr,FI2C_ENABLE_OFFSET,status);
        if (((input_32(addr,FI2C_ENABLE_STATUS_OFFSET)) & FI2C_IC_ENABLE_MASK) == status)
        {
            //!!!!!!!!!!!return ture;
        }
    }
    debug!("the enable is {:?}",enable);
    //!!!!!!!!!!!!!!!!!return false;
}

//计算I2C的上升沿下降沿配置
pub fn i2c_calc_timing(
    bus_clk_hz: u32,
    spk_cnt: u32,
    speed_cfg_p: &mut FI2cSpeedCfg
) -> bool {
    // 确保 speed_cfg_p 不为空
    assert!(speed_cfg_p.is_some());

    let speed_mode = speed_cfg_p.speed_mode;
    let info_p = &I2C_SPEED_CFG[speed_mode];
    
    let mut fall_cnt: i32;
    let mut rise_cnt: i32;
    let mut min_t_low_cnt: i32;
    let mut min_t_high_cnt: i32;
    let mut hcnt: i32;
    let mut lcnt: i32;
    let mut period_cnt: i32;
    let mut diff: i32;
    let mut tot: i32;
    let scl_rise_time_ns: i32;
    let scl_fall_time_ns: i32;

    period_cnt = bus_clk_hz / info_p.speed;
    scl_rise_time_ns = info_p.def_risetime_ns;
    scl_fall_time_ns = info_p.def_falltime_ns;

    // 将周期转换为 IC 时钟周期的数量
    fall_cnt = DIV_ROUND_UP(bus_clk_hz / 1000 * scl_rise_time_ns, NANO_TO_KILO);
    rise_cnt = DIV_ROUND_UP(bus_clk_hz / 1000 * scl_fall_time_ns, NANO_TO_KILO);
    min_t_low_cnt = DIV_ROUND_UP(bus_clk_hz / 1000 * info_p->min_scl_lowtime_ns, NANO_TO_KILO);
    min_t_high_cnt = DIV_ROUND_UP(bus_clk_hz / 1000 * info_p->min_scl_hightime_ns, NANO_TO_KILO);

    // 打印调试信息
    debug!(
        "i2c: mode {}, bus_clk {}, speed {}, period {} rise {} fall {} tlow {} thigh {} spk {}",
        speed_mode,
        bus_clk_hz,
        info_p.speed,
        period_cnt,
        rise_cnt,
        fall_cnt,
        min_t_low_cnt,
        min_t_high_cnt,
        spk_cnt
    );

    // 根据以下公式反推 hcnt 和 lcnt
    // SCL_High_time = [(HCNT + IC_*_SPKLEN + 7) * icClk] + SCL_Fall_time
    // SCL_Low_time = [(LCNT + 1) * icClk] - SCL_Fall_time + SCL_Rise_time
    hcnt = min_t_high_cnt - fall_cnt - 7 - spk_cnt as i32;
    lcnt = min_t_low_cnt - rise_cnt + fall_cnt - 1;

    if hcnt < 0 || lcnt < 0 {
        debug!("i2c: bad counts. hcnt = {} lcnt = {}", hcnt, lcnt);
        return false;
    }

    // 确保周期符合要求。如果不符合，将差值均分并偏向 lcnt
    tot = hcnt + lcnt + 7 + spk_cnt as i32 + rise_cnt + 1;

    if tot < period_cnt {
        diff = (period_cnt - tot) / 2;
        hcnt += diff;
        lcnt += diff;
        tot = hcnt + lcnt + 7 + spk_cnt as i32 + rise_cnt + 1;
        lcnt += period_cnt - tot;
    }

    speed_cfg_p.scl_lcnt = lcnt as u32;
    speed_cfg_p.scl_hcnt = hcnt as u32;
    speed_cfg_p.sda_hold = (bus_clk_hz / 1000 * 300 + NANO_TO_KILO - 1) / NANO_TO_KILO; // 使用默认值，除非另有指定

    // 打印最终配置
    debug!(
        "i2c: hcnt = {} lcnt = {} sda hold = {}",
        speed_cfg_p.scl_hcnt,
        speed_cfg_p.scl_lcnt,
        speed_cfg_p.sda_hold
    );

    true
}

//计算I2C的速度配置
// enum
// {
//     FI2C_STANDARD_SPEED = 0,
//     FI2C_FAST_SPEED,
//     FI2C_HIGH_SPEED,
//     FI2C_SPEED_MODE_MAX
// };
pub fn FI2cCalcSpeedCfg(addr: u32, speed: u32, bus_clk_hz: u32, speed_cfg_p: &mut FI2cSpeedCfg) -> bool {
    assert!(speed_cfg_p.is_some()); // 确保 speed_cfg_p 不为空
    let spk_cnt: u32;

    if speed >= 1000000 {
        speed_cfg_p.speed_mode = 2;
        spk_cnt = input_32(addr, 0xa4);
    } else if speed >= 400000 {
        speed_cfg_p.speed_mode = 1;
        spk_cnt = input_32(addr, 0xa0);
    } else if speed >= 100000 {
        speed_cfg_p.speed_mode = 0;
        spk_cnt = input_32(addr, 0xa0);
    } else {
        return false;
    }

    FI2cCalcTiming(bus_clk_hz, spk_cnt, speed_cfg_p)
}

//设置I2C控制器的速率
pub fn FI2cSetSpeed(addr: u32, speed_rate: u32) -> bool {
    let mut ret = 0;
    let mut speed_cfg = FI2cSpeedCfg { ..Default::default() }; // 初始化 speed_cfg
    let enable_status:u32;
    let mut reg_val:u32;

    // 计算速率配置
    ret = FI2cCalcSpeedCfg(addr, speed_rate, 50000000, &mut speed_cfg);
    if ret != 0 {
        return ret;
    }

    // 获取启用状态
    enable_status = input_32(addr,0x9C);

    // 重置速率模式位
    reg_val = (input_32(addr, 0x00) & !FI2C_CON_SPEED_MASK);
    match speed_cfg.speed_mode {
        0 => {
            reg_val |= (0x1 << 1);
            output_32(addr, 0x14, speed_cfg.scl_hcnt);
            output_32(addr, 0x18, speed_cfg.scl_lcnt);
        }
        1 => {
            reg_val |= (0x2 << 1);
            output_32(addr, 0x1C, speed_cfg.scl_hcnt);
            output_32(addr, 0x20, speed_cfg.scl_lcnt);
        }
        2 => {
            reg_val |= (0x3 << 1);
            output_32(addr, 0x24, speed_cfg.scl_hcnt);
            output_32(addr, 0x28, speed_cfg.scl_lcnt);
        }
        _ => {
            return false;
        }
    }

    output_32(addr, 0x00, reg_val);

    // 配置 SDA 保持时间（如果需要）
    if speed_cfg.sda_hold != 0 {
        output_32(addr, 0x7c, speed_cfg.sda_hold);
    }

    // 恢复 I2C 状态
    if enable_status == (0x1 << 0) {
        ret |= FI2cSetEnable(addr, true);
    }

    ret
}

//等待特定的I2C状态位直到状态不存在或者超时
pub fn FI2cWaitStatus(addr: u32, stat_bit: u32) -> false {
    let mut timeout:u32 = 0;

    // 等待状态位设置或超时
    while !((input_32(addr, 0x70) & stat_bit) != 0) && (50000 > timeout) {
        busy_wait(Duration::from_millis(1));// 等待 1 微秒
        timeout += 1;
    }

    if timeout >= 50000 {
        debug!("Timeout when wait status: {:?}", stat_bit);
        return false;
    }

    true
}

//等待I2C总线忙
pub fn FI2cWaitBusBusy(addr: u32) -> bool {
    let mut ret:u32 = 0;

    if (input_32(addr, 0x70) & (0x1 << 5)) &&
        (0 != FI2cWaitStatus(addr, (0x1 << 2)))
    {
        ret = 1111;
        debug!("Timeout when wait i2c bus busy.");
    }

    ret
}

//设置与I2C主机通信的从机地址
pub fn FI2cSetTar(addr: u32, tar_addr: u32) -> bool {
    let enable_status = input_32(addr,0x9C);
    let mut ret = 0;

    if enable_status == (0x1 << 0) {
        ret = FI2cSetEnable(addr, false);
    }

    if ret == 0 {
        output_32(addr, 0x04, tar_addr & FI2C_IC_TAR_MASK);
    }

    if enable_status == (0x1 << 0) {
        ret = FI2cSetEnable(addr, true);
    }

    ret
}

//从机模式下，设置I2C地址
pub fn FI2cSetSar(addr: u32, sar_addr: u32) -> false {
    let enable_status = input_32(addr,0x9C);
    let mut ret = 0;

    if enable_status == (0x1 << 0) {
        ret = FI2cSetEnable(addr, false);
    }

    if ret == 0 {
        output_32(addr, 0x08, sar_addr & FI2C_IC_SAR_MASK);
    }

    if enable_status == (0x1 << 0) {
        ret = FI2cSetEnable(addr, true);
    }

    ret
}

//等待接收Fifo传输完成
pub fn FI2cFlushRxFifo(addr: u32) -> false {
    let mut data: u8;
    let mut timeout = 0;
    let mut ret = 0;

    // 读取数据直到 FIFO 为空
    while (input_32(addr, 0x70) & (0x1 << 3)) != 0 {
        data = input_32(addr, 0x10) as u8;

        if timeout >= 50000{
            ret = 50000;
            debug!("Timeout when flush fifo.");
            break;
        }

        busy_wait(Duration::from_millis(1)); // 等待 1 微秒
        timeout += 1;
    }

    ret
}

//清除中断状态位，返回清除前的中断状态
pub fn FI2cClearIntrBits(addr: u32, last_err_p: &mut u32) -> u32 {
    assert!(last_err_p.is_some());

    let stat:u32 = input_32(addr, 0x2C);

    // 读取以清除中断状态位
    if (stat & (0x1 << 6)) != 0 {
        *last_err_p = input_32(addr, 0x80); // 读取中止源
        input_32(addr, 0x54); // 清除 TX_ABRT 中断
    }

    if (stat & (0x1 << 0)) != 0 {
        input_32(addr, 0x44); // 清除 RX_UNDER 中断
    }

    if (stat & (0x1 << 1)) != 0 {
        input_32(addr, 0x48); // 清除 RX_OVER 中断
    }

    if (stat & (0x1 << 3)) != 0 {
        input_32(addr, 0x4C); // 清除 TX_OVER 中断
    }

    if (stat & (0x1 << 7)) != 0 {
        input_32(addr, 0x58); // 清除 RX_DONE 中断
    }

    if (stat & (0x1 << 8)) != 0 {
        input_32(addr, 0x5c); // 清除 ACTIVITY 中断
    }

    if (stat & (0x1 << 9)) != 0 {
        input_32(addr, 0x60); // 清除 STOP_DET 中断
    }

    if (stat & (0x1 << 10)) != 0 {
        input_32(addr, 0x64); // 清除 START_DET 中断
    }

    if (stat & (0x1 << 11)) != 0 {
        input_32(addr, 0x68); // 清除 GEN_CALL 中断
    }

    stat
}

//
pub fn FI2cClearAbort(addr: u32) {
    let mut reg_val:u32;
    let mut timeout:u32 = 50000;

    loop {
        // 清除中断状态
        input_32(addr, 0x40);
        reg_val = input_32(addr, 0x80);

        if reg_val == 0 {
            return;
        }

        if timeout == 0 {
            debug!("Timeout when clear abort.");
            return;
        }

        timeout -= 1;
        busy_wait(Duration::from_millis(1)); // 等待 1 微秒
    }
}














