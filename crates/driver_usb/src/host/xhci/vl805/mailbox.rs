use core::{
    mem::size_of,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
};
use axhal::time;
use alloc::vec::{self, Vec};
use axhal::mem::phys_to_virt;
use log::{debug, info};
 use core::time::Duration;


pub const BCM_MAILBOX_PROP_OUT: u32 = 8;
const GPU_MEM_BASE: usize = 0xC0000000;
const ARM_IO_BASE: usize = 0xFE000000;
const MAIL_BOX_BASE: usize = ARM_IO_BASE + 0xB880;
const MAILBOX_STATUS_EMPTY: u32 = 0x40000000;
const MAILBOX_STATUS_FULL: u32 = 0x80000000;
const MAILBOX0_READ: usize = MAIL_BOX_BASE + 0x00;
const MAILBOX0_STATUS: usize = MAIL_BOX_BASE + 0x18;
const MAILBOX1_WRITE: usize = MAIL_BOX_BASE + 0x20;
const MAILBOX1_STATUS: usize = MAIL_BOX_BASE + 0x38;




pub struct Mailbox {
    n_channel: u32,
}

impl Mailbox {
    pub fn new() -> Self {
        return Self {
            n_channel: BCM_MAILBOX_PROP_OUT,
        };
    }

    pub fn send(self, msg: &impl RaspiMsg, dma: &mut [u8])  {
        msg.write_to(dma);
        unsafe {
            let mut send_addr = dma.as_ptr() as usize;
            send_addr = bus_address(send_addr);
            debug!("send msg to {:X}", send_addr);

            let result = self.write_read((send_addr as u32));

            // barrier::dmb(SY);

            debug!("read: 0x{:X}", result);
            debug!("waiting for response...");
            unsafe{
                let buff =  &*slice_from_raw_parts(dma.as_ptr() as *const u32, dma.len() / 4);
                let res = dma.as_ptr().offset(4) as *const PropertyCode;

                while res.read_volatile() == PropertyCode::Request {}
                let res = res.read_volatile();
                debug!("response: {:?}", res);
                
                if res != PropertyCode::ResponseSuccess{
                    panic!("mailbox fail");
                }
            }
        }
    }

    fn read(&self) -> u32 {
        while read32(MAILBOX0_STATUS) == MAILBOX_STATUS_EMPTY {
            //println!("Mailbox is empty");
        }

        loop {
            let r = read32(MAILBOX0_READ);
            debug!("mailbox read 0x{:X}", r);
            if (r & 0xf) == self.n_channel {
                return r & !0xf;
            }
        }
    }

    fn write(&self, data: u32) -> () {
        while read32(MAILBOX1_STATUS) == MAILBOX_STATUS_FULL {
            //println!("Mailbox is full");
        }
        let w = data | self.n_channel;
        debug!("mailbox write 0x{:X}", w);
        write32(MAILBOX1_WRITE, w);
    }

    fn flush(&self) {
        loop {
            let r = read32(MAILBOX0_STATUS);
            if r == MAILBOX_STATUS_EMPTY {
                return;
            }
            read32(MAILBOX0_READ);
            time::busy_wait(Duration::from_millis(20));
        }
    }

    fn write_read(&self, data: u32) -> u32 {
        self.flush();
        debug!("flush ok");
        self.write(data);
        while read32(MAILBOX1_STATUS) != MAILBOX_STATUS_EMPTY {
            //println!("Mailbox is full");
        }
        debug!("write ok");
        self.read()
    }
}

fn bus_address(addr: usize) -> usize {
    // addr | GPU_MEM_BASE
    (addr & !GPU_MEM_BASE) | GPU_MEM_BASE
}

pub trait RaspiMsg {
    const ID: PropTag;
    fn __tag_bytes(&self) -> Vec<u8>;

    fn write_to(&self, buff: &mut [u8]) {
        let mut data: Vec<u32> = alloc::vec![];
        data.push(0); // size
        data.push(0); // request
        data.push(Self::ID as _); // id

        let tag = self.__tag_bytes();
        let tag32_len = tag.len().div_ceil(size_of::<u32>());
        let tag_len = tag32_len * size_of::<u32>();
        data.push(tag_len as _); // tag value len
        data.push(0); // tag request
        let last = data.len() - 1;

        // tag value
        unsafe {
            for _ in 0..tag32_len {
                data.push(0);
            }
            let tag_value = &mut *slice_from_raw_parts_mut(
                data.as_mut_ptr().offset(last as _) as *mut u8,
                tag.len(),
            );

            tag_value.copy_from_slice(tag.as_slice());
        }
    
        data.push(0); // end tag
        data[0] = (data.len() * 4) as _;

        unsafe {
            let ptr = data.as_ptr() as *const u8;
            let ptr = &*slice_from_raw_parts(ptr, data.len() * 4);
            buff[0..ptr.len()].copy_from_slice(ptr);
        }
    }
}

pub struct MsgNotifyXhciReset {}

impl RaspiMsg for MsgNotifyXhciReset {
    const ID: PropTag = PropTag::NotifyXhciReset;

    fn __tag_bytes(&self) -> Vec<u8> {
        let mut data: Vec<u8> = alloc::vec![0; 4];
        unsafe {
            let ptr = data.as_ptr() as *mut u32;
            // 树莓派写死了固定地址 bus 1, device 0, func 0
            *ptr = 0x100000;
        }
        data
    }
}


pub struct MsgGetFirmwareRevision {}
impl RaspiMsg for MsgGetFirmwareRevision {
    const ID: PropTag = PropTag::GetFirmwareRevision;

    fn __tag_bytes(&self) -> Vec<u8> {
        alloc::vec![]
    }
}



#[repr(u32)]
#[derive(Debug)]
pub enum PropTag {
    NotifyXhciReset = 0x00030058,
    GetFirmwareRevision = 0x1,
    GetBoardModel = 0x00010001,
}
#[repr(u32)]
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyCode {
    Request = 0x00000000,
    ResponseSuccess = 0x80000000,
    ResponseFailure = 0x80000001,
}

fn read32(addr: usize) -> u32 {
    let vaddr = phys_to_virt(addr.into());
    unsafe {
        // *(vaddr.as_ptr() as *const u32)
        // u32::from_le((vaddr.as_mut_ptr() as *const u32).read_volatile())
        (vaddr.as_mut_ptr() as *const u32).read_volatile()
    }
}
fn write32(addr: usize, data: u32) -> () {
    let vaddr = phys_to_virt(addr.into());
    unsafe { (vaddr.as_mut_ptr() as *mut u32).write_volatile(data) }
}
