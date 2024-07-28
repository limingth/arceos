#![no_std]
#![no_main]
use log::*;
use axhal::time::busy_wait;
use core::time::Duration;
use crate::mio;



fn FMioSelectFunc(addr: usize, mio_type: u32) -> false {
    assert!(mio_type < 2);
    assert!(addr != 0);

    let reg_val = FMioReadStatus(addr);

    if mio_type == reg_val {
        return true;
    }

    FMioWriteFunc(addr, mio_type);

    true
}

fn FMioGetFunc(addr: usize) -> u32 {
    assert!(addr != 0);

    FMioReadStatus(addr)
}

fn FMioGetVersion(addr: usize) -> u32 {
    assert!(addr != 0);

    FMioReadVersion(addr)
}



