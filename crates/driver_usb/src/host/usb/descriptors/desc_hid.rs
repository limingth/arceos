use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub struct Hid {}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum USBHIDSubClassDescriptorType {
    None = 0,
    KeyBoard = 1,
    Mouse = 2,
    Other,
}
