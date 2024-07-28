#![no_std]
#![no_main]
use log::*;
use core::ptr;
use core::ptr::write_volatile;

fn write_reg(addr: u32, value: u32) {
    debug!("Writing value {:#X} to address {:#X}", value, addr);
    *(addr as *mut u32) = value;
}

fn read_reg(addr: u32) -> u32 {
    let value = *(addr as *const u32);
    debug!("Read value {:#X} from address {:#X}", value, addr);
    value
}

fn input_32(addr:u32,offset:usize,) -> u32{
    let address:32=addr+offset as u32;
    read_reg(address)
}

fn output_32(addr:u32,offset:usize,value:u32){
    let address:u32=addr+offset as u32;
    write_reg(address,value);
}

