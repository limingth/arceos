use num_derive::{FromPrimitive, ToPrimitive};

#[derive(FromPrimitive, ToPrimitive, Copy, Clone, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum DescriptorType {
    //USB 1.1: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    // USB 2.0: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    DeviceQualifier = 6,
    OtherSpeedConfiguration = 7,
    InterfacePower1 = 8,
    Hid = 0x21,
    HIDReport = 0x22,
    HIDPhysical = 0x23,
    // USB 3.0+: 9.4 Standard Device Requests, Table 9-5. Descriptor Types
    OTG = 0x09,
    Debug = 0x0a,
    InterfaceAssociation = 0x0b,
    Bos = 0x0f,
    DeviceCapability = 0x10,
    SuperSpeedEndpointCompanion = 0x30,
    SuperSpeedPlusIsochEndpointCompanion = 0x31,
}
impl DescriptorType {
    pub(crate) fn forLowBit(self, index: u8) -> DescriptionTypeIndexPairForControlTransfer {
        DescriptionTypeIndexPairForControlTransfer { ty: self, i: index }
    }
}
pub(crate) struct DescriptionTypeIndexPairForControlTransfer {
    ty: DescriptorType,
    i: u8,
}

impl DescriptionTypeIndexPairForControlTransfer {
    pub(crate) fn bits(self) -> u16 {
        (self.ty as u16) << 8 | u16::from(self.i)
    }
}
