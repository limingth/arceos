use num_derive::{FromPrimitive, ToPrimitive};

pub struct ControlTransfer {
    request_type: bmRequestType,
    request: bRequest,
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum bRequest {
    GetStatus = 0,
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    GetDescriptor = 6,
    SetDescriptor = 7,
    GetConfiguration = 8,
    SetConfiguration = 9,
    GetInterface = 10,
    SetInterface = 11,
    SynchFrame = 12,
    SetEncryption = 13,
    GetEncryption = 14,
    SetHandshake = 15,
    GetHandshake = 16,
    SetConnection = 17,
    SetSecurityData = 18,
    GetSecurityData = 19,
    SetWusbData = 20,
    LoopbackDataWrite = 21,
    LoopbackDataRead = 22,
    SetInterfaceDs = 23,
    GetFwStatus = 26,
    SetFwStatus = 27,
    SetSel = 48,
    SetIsochDelay = 49,
    RESERVED,
}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
pub struct bmRequestType {
    direction: DataTransferDirection,
    transfer_type: DataTransferType,
    recipient: Recipient,
}

impl bmRequestType {
    pub fn new(
        direction: DataTransferDirection,
        transfer_type: DataTransferType,
        recipient: Recipient,
    ) -> bmRequestType {
        bmRequestType {
            direction,
            transfer_type,
            recipient,
        }
    }
}

impl Into<u8> for bmRequestType {
    fn into(self) -> u8 {
        (self.direction as u8) << 7 | (self.transfer_type as u8) << 4 | self.recipient as u8
    }
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[repr(u8)]
pub enum DataTransferDirection {
    Host2Device = 0,
    Device2Host = 1,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[repr(u8)]
pub enum DataTransferType {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Reserved = 3,
}

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[repr(u8)]
pub enum Recipient {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
    Reserved,
}
