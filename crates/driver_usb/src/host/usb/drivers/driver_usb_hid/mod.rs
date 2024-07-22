use core::{marker::PhantomData, time::Duration};

use alloc::vec::Vec;
use alloc::{collections::BTreeMap, fmt::format, string::String, sync::Arc};
use alloc::{format, vec};
use axhal::cpu::this_cpu_id;
use axhal::{paging::PageSize, time::busy_wait};
use axtask::sleep_until;
use driver_common::BaseDriverOps;
use log::{debug, info};
use num_traits::{FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::ring::trb::command;
use xhci::ring::trb::transfer::{self, Direction, Normal, TransferType};

use crate::host::xhci::ring::Ring;
use crate::{
    ax::USBDeviceDriverOps,
    dma::DMA,
    host::{
        usb::descriptors::{
            self,
            desc_device::{Device, USBDeviceClassCode},
            desc_hid::{Hid, USBHIDProtocolDescriptorType, USBHIDSubclassDescriptorType},
            DescriptionTypeIndexPairForControlTransfer, Descriptor, DescriptorType,
        },
        xhci::{xhci_device::DeviceAttached, Xhci},
    },
    OsDep,
};

const TAG: &str = "[USB-HID DRIVER]";

pub fn fallback_solve_hid_mouse_report(report_buffer: &Vec<u8>) {
    if report_buffer.iter().any(|b| *b != 0) {
        if report_buffer[0] != 0 {
            match report_buffer[0] {
                1 => {
                    info!("left button press");
                }
                2 => {
                    info!("right button press");
                }
                3 => {
                    info!("both button press");
                }
                4 => {
                    info!("middle button press");
                }
                _ => {}
            }
        } else {
            let x_move =
                (((report_buffer[2] as u16) << 8 | report_buffer[3] as u16) & 0xfffeu16) as i16;
            let y_move =
                (((report_buffer[4] as u16) << 8 | report_buffer[5] as u16) & 0xfffeu16) as i16; //0xfffe:eliminate inaccuracy, so drop last bit to prevent just move 1 px
            if (x_move != 0) {
                debug!("x moved:{x_move}")
            }
            if (y_move != 0) {
                debug!("y moved:{y_move}")
            }
            if report_buffer[6] == 255u8 {
                debug!("wheel down!");
            } else if report_buffer[6] == 1u8 {
                debug!("wheel up!");
            }
        }
    } else {
    }
}
