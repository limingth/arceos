#![no_std]
#![no_main]

use axhal::time::busy_wait;
use core::default;
use core::ptr;
use core::slice;
use core::time::Duration;
use log::debug;
pub mod driver_iic;
pub mod driver_mio;
pub mod example;

use crate::driver_iic::i2c::*;
use crate::driver_iic::i2c_hw::*;
use crate::driver_iic::i2c_intr::*;
use crate::driver_iic::i2c_master::*;
use crate::driver_iic::i2c_sinit::*;
use crate::driver_iic::io::*;

use crate::driver_mio::mio::*;
use crate::driver_mio::mio_g::*;
use crate::driver_mio::mio_hw::*;
use crate::driver_mio::mio_sinit::*;

use crate::example::*;

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
        let mut ret: bool = true;
        let address: u32 = 0x3c;
        let mut speed_rate: u32 = 100000; /*kb/s*/
        FIOPadCfgInitialize(&mut iopad_ctrl, &FIOPadLookupConfig(0).unwrap());
        ret = FI2cMioMasterInit(address, speed_rate);
        if ret != true {
            debug!("FI2cMioMasterInit mio_id {:?} is error!", 1);
        }
        let offset: u32 = 0x05;
        let input: &[u8] = b"012";
        let input_len: u32 = input.len() as u32;
        let mut write_buf: [u8; 256] = [0; 256];
        let mut read_buf: [u8; 256] = [0; 256];


        unsafe {
            // 复制数据到 write_buf
            ptr::copy_nonoverlapping(input.as_ptr(), write_buf.as_mut_ptr(), input_len as usize);
            debug!("------------------------------------------------------");
            debug!("write 0x{:x} len {}", offset, input_len);

            // 调用 FI2cMasterwrite
            let ret = FI2cMasterWrite(&mut write_buf, input_len, offset);
            if ret != true {
                debug!("FI2cMasterwrite error!");
                return;
            }
            debug!("------------------------------------------------------");
            // 调用 FI2cSlaveDump
            FI2cSlaveDump();
            let read_buf =
                unsafe { slice::from_raw_parts_mut(read_buf.as_mut_ptr(), read_buf.len()) };

            // 调用 FI2cMasterRead
            let ret = FI2cMasterRead(read_buf, input_len, offset);
            debug!("------------------------------------------------------");
            if ret == true {
                debug!("Read {:?} len {:?}: {:?}.", offset, input_len, read_buf);
                FtDumpHexByte(read_buf.as_ptr(), input_len as usize);
            }
            debug!("------------------------------------------------------");
        }
    }
}
