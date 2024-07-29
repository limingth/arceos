use num_derive::{FromPrimitive, ToPrimitive};

pub mod hid_mouse;

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[repr(u8)]
pub enum USBHidDeviceSubClassCode {
    Mouse = 1,
}
