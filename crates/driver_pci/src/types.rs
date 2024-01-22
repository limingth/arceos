use tock_registers::interfaces::ReadWriteable;
use tock_registers::interfaces::Readable;
use tock_registers::interfaces::Writeable;
use tock_registers::registers::ReadOnly;
use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};
use crate::Address;

register_bitfields![
    u32,

    RC_CFG_REGS1 [
        VENDOR_ID OFFSET(0) NUMBITS(16) [],
        DEVICE_ID OFFSET(16) NUMBITS(16) [],
    ],

    RC_CFG_REGS2 [
        COMMAND OFFSET(0) NUMBITS(16) [],
        STATUS OFFSET(16) NUMBITS(16) [],
    ],

    RC_CFG_REGS3 [
        REVISION OFFSET(0) NUMBITS(8)[

        ],
        CLASS_CODE OFFSET(8) NUMBITS(24)[

        ]
    ],
    RC_CFG_REGS4 [
        HEADER_TYPE OFFSET(16) NUMBITS(7)[
            Endpoint = 0,
            PciPciBridge = 1,
            CardBusBridge = 3,
        ],
        HAS_MULTIPLE_FUNCTIONS OFFSET(23) NUMBITS(1)[
            False = 0,
            True = 1,
        ]
    ],

    RC_CFG_BUS_NUMS_REG1 [
        PRIMARY_BUS_NUMBER OFFSET(0) NUMBITS(8) [],
        SECONDARY_BUS_NUMBER OFFSET(8) NUMBITS(8) [],
        SUBORDINATE_BUS_NUMBER OFFSET(16) NUMBITS(8) [],
    ],
];

register_structs! {
    HeaderRegs {
        (0x00 => reg1: ReadOnly<u32, RC_CFG_REGS1::Register>),
        (0x04 => reg2: ReadOnly<u32, RC_CFG_REGS2::Register>),
        (0x08 => reg3: ReadOnly<u32, RC_CFG_REGS3::Register>),
        (0x0c => reg4: ReadOnly<u32, RC_CFG_REGS4::Register>),
        (0x10 => @END),
    }
}

register_structs! {
    PCIBridgeRegs {
        (0x00 => _rsvd1),
        (0x18 => primary_bus_number: ReadWrite<u8>),
        (0x19 => secondary_bus_number: ReadWrite<u8>),
        (0x1a => subordinate_bus_number: ReadWrite<u8>),
        (0x1b => @END),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderType {
    Endpoint,
    PciPciBridge,
    CardBusBridge,
    Unknown(u8),
}

#[derive(Clone)]
pub struct PciHeader {
    cfg_base: usize,
}

impl PciHeader {
    pub fn new(cfg_base: usize) -> PciHeader {
        PciHeader { cfg_base }
    }

    fn regs(&self) -> &'static HeaderRegs {
        unsafe { &*(self.cfg_base as *const HeaderRegs) }
    }

    pub fn vendor_id_and_device_id(&self) -> (u16, u16) {
        let regs = self.regs();
        (
            regs.reg1.read(RC_CFG_REGS1::VENDOR_ID) as u16,
            regs.reg1.read(RC_CFG_REGS1::DEVICE_ID) as u16,
        )
    }

    pub fn has_multiple_functions(&self) -> bool {
        match self
            .regs()
            .reg4
            .read_as_enum(RC_CFG_REGS4::HAS_MULTIPLE_FUNCTIONS)
        {
            Some(RC_CFG_REGS4::HAS_MULTIPLE_FUNCTIONS::Value::True) => true,
            _ => false,
        }
    }

    pub fn header_type(&self) -> HeaderType {
        match self.regs().reg4.read_as_enum(RC_CFG_REGS4::HEADER_TYPE) {
            Some(RC_CFG_REGS4::HEADER_TYPE::Value::Endpoint) => HeaderType::Endpoint,
            Some(RC_CFG_REGS4::HEADER_TYPE::Value::PciPciBridge) => HeaderType::PciPciBridge,
            Some(RC_CFG_REGS4::HEADER_TYPE::Value::CardBusBridge) => HeaderType::CardBusBridge,
            None => HeaderType::Unknown(0),
        }
    }
}

pub struct ConifgPciPciBridge {
    cfg_addr: usize,
}

impl ConifgPciPciBridge {
    pub fn new(cfg_addr: usize) -> ConifgPciPciBridge {
        ConifgPciPciBridge { cfg_addr }
    }

    fn regs(&self) -> &'static PCIBridgeRegs {
        unsafe { &*(self.cfg_addr as *const PCIBridgeRegs) }
    }


    pub fn set_primary_bus_number(&self, bus: u8) {
        self.regs().primary_bus_number.set(bus);
    }

    pub fn set_secondary_bus_number(&self, bus: u8) {
        self.regs().secondary_bus_number.set(bus);
    }

    pub fn set_subordinate_bus_number(&self, bus: u8) {
        self.regs().subordinate_bus_number.set(bus);
    }
}
