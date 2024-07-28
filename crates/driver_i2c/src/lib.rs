#![no_std] 
#![no_main]
use log::debug;
const MIO1_BASE: u32 = 0x000_2801_6000;
const IC_CON: usize = 0x00;
const IC_TAR: usize = 0x04;
const IC_DATA_CMD: usize = 0x10;
const IC_ENABLE: usize = 0x6C;
const IC_STATUS: usize = 0x70;
const CREG_MIO_FUNC_SEL_OFFSET: usize = 0x00;
use crates::driver_iic::{i2c_hw,i2c_init,i2c,i2c_sinit,i2c_master,io};
use crates::driver_mio::{mio_g,mio_hw,mio_sinit,mio};

FI2cCalcSpeedCfg

unsafe fn write_reg(addr: u32, value: u32) {
    debug!("Writing value {:#X} to address {:#X}", value, addr);
    *(addr as *mut u32) = value;
}

unsafe fn read_reg(addr: u32) -> u32 {
    let value = *(addr as *const u32);
    debug!("Read value {:#X} from address {:#X}", value, addr);
    value
}

unsafe fn configure_mio_for_i2c(mio_base: u32) {
    let creg_mio_func_sel = mio_base + CREG_MIO_FUNC_SEL_OFFSET as u32;
    
    debug!("Configuring MIO for I2C at base {:#X}", mio_base);
    write_reg(creg_mio_func_sel, 0x00); 
}

unsafe fn wait_send_fifo_not_full(mio_base: u32) {
    let ic_status = mio_base + IC_STATUS as u32;
    while (read_reg(ic_status) & (1 << 1)) == 0 {}
}

unsafe fn i2c_init(mio_base: u32, slave_address: u8) {
    let ic_enable = mio_base + IC_ENABLE as u32;
    let ic_con = mio_base + IC_CON as u32;
    let ic_tar = mio_base + IC_TAR as u32;

    debug!("Initializing I2C at base {:#X} with slave address {:#X}", mio_base, slave_address);
    write_reg(ic_enable, 0x00);
    write_reg(ic_con, 0x63);
    write_reg(ic_tar, slave_address as u32);
    write_reg(ic_enable, 0x01);
}

unsafe fn i2c_send_data(mio_base: u32, data: &[u8]) {
    let ic_status = mio_base + IC_STATUS as u32;
    let ic_data_cmd = mio_base + IC_DATA_CMD as u32;

    debug!("Sending I2C data: {:?}", data);

    for (i, &byte) in data.iter().enumerate() {
        wait_send_fifo_not_full(mio_base);
        if i == data.len() - 1 {
            debug!("writing end!");
            write_reg(ic_data_cmd, (byte as u32) |(1 << 9));  
        } else {
            write_reg(ic_data_cmd, byte as u32);  
        }
    }
}

unsafe fn i2c_receive_data(mio_base: u32, buffer: &mut [u8]) {
    let ic_status = mio_base + IC_STATUS as u32;
    let ic_data_cmd = mio_base + IC_DATA_CMD as u32;
    debug!("Receiving I2C data into buffer of length {}", buffer.len());
    for i in 0..buffer.len() {
        if i == buffer.len() - 1 {
            write_reg(ic_data_cmd, (1 << 8) | (1 << 9)); 
        } else {
            write_reg(ic_data_cmd, (1 << 8)); 
        }
    }
    for (i, byte) in buffer.iter_mut().enumerate() {
        while (read_reg(ic_status) & (1 << 3)) == 0 {}
        *byte = read_reg(ic_data_cmd) as u8;
    }
}

// unsafe fn oled_fill_screen(mio_base: u32) {
//     for y in 0..8 {
//         oled_set_cursor(mio_base, y, 0);
//         for x in 0..128 {
//             oled_write_data(mio_base, 0xFF);  // 0xFF表示全亮
//         }
//     }
// }

// // OLED 特定的命令和数据发送函数
// unsafe fn oled_i2c_send_byte(mio_base: u32, byte: u8) {
//     debug!("Sending OLED byte {:#X}", byte);
//     i2c_send_data(mio_base, &[byte]);
// }

// unsafe fn oled_write_command(mio_base: u32, command: u8) {
//     debug!("Writing OLED command {:#X}", command);
//     oled_i2c_send_byte(mio_base, 0x78);  // 从机地址
//     oled_i2c_send_byte(mio_base, 0x00);  // 写命令
//     oled_i2c_send_byte(mio_base, command);
// }

// unsafe fn oled_write_data(mio_base: u32, data: u8) {
//     debug!("Writing OLED data {:#X}", data);
//     oled_i2c_send_byte(mio_base, 0x78);  // 从机地址
//     oled_i2c_send_byte(mio_base, 0x40);  // 写数据
//     oled_i2c_send_byte(mio_base, data);
// }

// unsafe fn oled_set_cursor(mio_base: u32, y: u8, x: u8) {
//     debug!("Setting OLED cursor to ({}, {})", y, x);
//     oled_write_command(mio_base, 0xB0 | y);  // 设置Y位置
//     oled_write_command(mio_base, 0x10 | ((x & 0xF0) >> 4));  // 设置X位置高4位
//     oled_write_command(mio_base, 0x00 | (x & 0x0F));  // 设置X位置低4位
// }

// unsafe fn oled_clear(mio_base: u32) {
//     debug!("Clearing OLED display");
//     for j in 0..8 {
//         oled_set_cursor(mio_base, j, 0);
//         for _ in 0..128 {
//             oled_write_data(mio_base, 0x00);
//         }
//     }
// }

// fn get_char_font(c: char) -> [u8; 8] {
//     match c {
//         'h' => [0x00, 0x7F, 0x08, 0x08, 0x08, 0x08, 0x70, 0x00],
//         'e' => [0x00, 0x3E, 0x49, 0x49, 0x49, 0x41, 0x00, 0x00],
//         'l' => [0x00, 0x41, 0x7F, 0x40, 0x00, 0x00, 0x00, 0x00],
//         'o' => [0x00, 0x3E, 0x41, 0x41, 0x41, 0x3E, 0x00, 0x00],
//         'r' => [0x00, 0x7F, 0x08, 0x08, 0x08, 0x30, 0x00, 0x00],
//         'u' => [0x00, 0x3F, 0x40, 0x40, 0x40, 0x3F, 0x00, 0x00],
//         's' => [0x00, 0x32, 0x49, 0x49, 0x49, 0x26, 0x00, 0x00],
//         't' => [0x00, 0x01, 0x01, 0x7F, 0x01, 0x01, 0x00, 0x00],
//         _ => [0x00; 8], // 默认未定义字符为空
//     }
// }

// unsafe fn oled_show_char(mio_base: u32, line: u8, column: u8, char: char) {
//     debug!("Showing char '{}' at line {}, column {}", char, line, column);
//     let font = get_char_font(char);
//     oled_set_cursor(mio_base, (line - 1) * 2, (column - 1) * 8);
//     for i in 0..8 {
//         oled_write_data(mio_base, font[i]);
//     }
//     oled_set_cursor(mio_base, (line - 1) * 2 + 1, (column - 1) * 8);
//     for i in 0..8 {
//         oled_write_data(mio_base, font[i]);
//     }
// }

// unsafe fn oled_show_string(mio_base: u32, line: u8, column: u8, string: &str) {
//     debug!("Showing string '{}' at line {}, column {}", string, line, column);
//     for (i, char) in string.chars().enumerate() {
//         oled_show_char(mio_base, line, column + i as u8, char);
//     }
// }

// unsafe fn oled_init(mio_base: u32) {
//     debug!("Initializing OLED at base {:#X}", mio_base);
//     for _ in 0..1_000_000 {
//         // 上电延时
//     }
    
//     i2c_init(mio_base, 0x78);
//     oled_write_command(mio_base, 0xAE);  // 关闭显示

//     oled_write_command(mio_base, 0x00);  // 设置低列地址
//     oled_write_command(mio_base, 0x10);  // 设置高列地址

//     oled_write_command(mio_base, 0x40);  // 设置起始行地址

//     oled_write_command(mio_base, 0x81);  // 设置对比度控制寄存器
//     oled_write_command(mio_base, 0xFF);

//     oled_write_command(mio_base, 0xA1);  // 设置段重新映射
//     oled_write_command(mio_base, 0xA6);  // 设置正常显示

//     oled_write_command(mio_base, 0xA8);  // 设置多路复用比率
//     oled_write_command(mio_base, 0x3F);  // 1/64占空比

//     oled_write_command(mio_base, 0xC8);  // 设置COM输出扫描方向

//     oled_write_command(mio_base, 0xD3);  // 设置显示偏移
//     oled_write_command(mio_base, 0x00);  // 无偏移

//     oled_write_command(mio_base, 0xD5);  // 设置显示时钟分频比/振荡器频率
//     oled_write_command(mio_base, 0x80);

//     oled_write_command(mio_base, 0xD8);  // 设置预充电周期
//     oled_write_command(mio_base, 0x05);

//     oled_write_command(mio_base, 0xD9);  // 设置COM引脚硬件配置
//     oled_write_command(mio_base, 0xF1);

//     oled_write_command(mio_base, 0xDA);  // 设置VCOMH
//     oled_write_command(mio_base, 0x30);

//     oled_write_command(mio_base, 0x8D);  // 设置电荷泵
//     oled_write_command(mio_base, 0x14);

//     oled_write_command(mio_base, 0xAF);  // 打开显示
//     oled_clear(MIO1_BASE);
// }

pub fn run_iicoled() {
    unsafe {
        configure_mio_for_i2c(MIO1_BASE);
        i2c_init(MIO1_BASE, 0x3C); 
        let send_data = [0x55, 0xAA, 0xF0];
        debug!("Sending data: {:?}", send_data);
        i2c_send_data(MIO1_BASE, &send_data);
        let mut receive_buffer = [0u8; 3];
        debug!("Receiving data into buffer...");
        i2c_receive_data(MIO1_BASE, &mut receive_buffer);
        debug!("Received data: {:?}", receive_buffer);
    }
}


// debug!("Running I2C OLED example");
    // unsafe {
    //     configure_mio_for_i2c(MIO1_BASE);
    //     oled_init(MIO1_BASE);
    // }
    // while true {
    //     unsafe {
    //         oled_fill_screen(MIO1_BASE);
    //     }
    // }


