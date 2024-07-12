use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub struct Hid {
    pub len: u8,
    pub descriptor_type: u8,
    pub hid_bcd: u16,
    pub country_code: u8,
    pub num_descriptions: u8,
    pub report_descriptor_type: u8, // actually, these two entry is a variant length vector, but we only pick first entry!
    pub report_descriptor_len: u16, //
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
pub enum USBHIDSubclassDescriptorType {
    None = 0,
    BootInterface = 1,
    Reserved,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum USBHIDProtocolDescriptorType {
    None = 0,
    KeyBoard = 1,
    Mouse = 2,
    Reserved,
}
