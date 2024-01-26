use tock_registers::interfaces::ReadWriteable;
use tock_registers::interfaces::Readable;
use tock_registers::interfaces::Writeable;
use tock_registers::registers::ReadOnly;
use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};
use crate::PciAddress;

register_bitfields![
    u32,

    RC_CFG_REGS1 [
        VENDOR_ID OFFSET(0) NUMBITS(16) [],
        DEVICE_ID OFFSET(16) NUMBITS(16) [],
    ],

    RC_CFG_REGS3 [
        REVISION OFFSET(0) NUMBITS(8)[],
        INTERFACE OFFSET(8) NUMBITS(8)[],
        SUB_CLASS OFFSET(16) NUMBITS(8)[],
        BASE_CLASS OFFSET(24) NUMBITS(8)[],
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

register_bitfields! {
    u16,
    RC_CFG_COMMAND[
        IO_SPACE_ENABLE OFFSET(0) NUMBITS(1) [],
        MEMORY_SPACE_ENABLE OFFSET(1) NUMBITS(1) [],
        BUS_MASTER_ENABLE OFFSET(2) NUMBITS(1) [],
        SPECIAL_CYCLE_ENABLE OFFSET(3) NUMBITS(1) [],
        MEMORY_WRITE_AND_INVALIDATE OFFSET(4) NUMBITS(1) [],
        VGA_PALETTE_SNOOP OFFSET(5) NUMBITS(1) [],
        PARITY_ERROR_RESPONSE OFFSET(6) NUMBITS(1) [],
        IDSEL_STEP_WAIT_CYCLE_CONTROL OFFSET(7) NUMBITS(1) [],
        SERR_ENABLE OFFSET(8) NUMBITS(1) [],
        FAST_BACK_TO_BACK_ENABLE OFFSET(9) NUMBITS(1) [],
        INTERRUPT_DISABLE OFFSET(10) NUMBITS(1) [],
    ],
 
    RC_CFG_STATUS[
        IMMEDIATE_READINESS OFFSET(0) NUMBITS(3) [],
        INTERRUPT_STATUS OFFSET(3) NUMBITS(1) [],
        CAPABILITIES_LIST OFFSET(4) NUMBITS(1) [],
        CAPABLE_66MHZ OFFSET(5) NUMBITS(1) [],
    ],
}

register_structs! {
    HeaderRegs {
        (0x00 => reg1: ReadOnly<u32, RC_CFG_REGS1::Register>),
        (0x04 => command: ReadWrite<u16, RC_CFG_COMMAND::Register>),
        (0x06 => status: ReadOnly<u16, RC_CFG_STATUS::Register>),
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
        (0x1b => secondary_latency_timer: ReadWrite<u8>),
        (0x1c => _io),
        (0x20 => memory_base: ReadWrite<u16>),
        (0x22 => memory_limit: ReadWrite<u16>),
        (0x24 => _rsvd2),
        (0x3C => _interrupt_line),
        (0x3D => interrupt_pin),
        (0x3E => control),
        (0x40 => @END),
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
    pub fn revision_and_class(&self)-> (Revision, BaseClass, SubClass, Interface){
        let reg3 = &self.regs().reg3;
        return (
            reg3.read(RC_CFG_REGS3::REVISION) as u8,
            reg3.read(RC_CFG_REGS3::BASE_CLASS) as u8,
            reg3.read(RC_CFG_REGS3::SUB_CLASS) as u8,
            reg3.read(RC_CFG_REGS3::INTERFACE) as u8,
        );
    }
    pub fn set_command(&self, command: &[ConfigCommand]) {
        let cmd = command.iter().fold(0u16, |acc, a|{ acc + a.clone() as u16});
        self.regs().command.set(cmd)
    }
}


pub type Revision = u8;
pub type BaseClass = u8;
pub type SubClass = u8;
pub type Interface = u8;



#[derive(Clone, Copy, Debug)]
#[repr(u16)]
pub enum ConfigCommand {
    IoSpaceEnable = 1 << 0,
    MemorySpaceEnable = 1 << 1,
    BusMasterEnable = 1 << 2,
    SpecialCycleEnable = 1 << 3,
    MemoryWriteAndInvalidate = 1 << 4,
    VGAPaletteSnoop = 1 << 5,
    ParityErrorResponse = 1 << 6,
    IDSELStepWaitCycleControl = 1 << 7,
    SERREnable = 1 << 8,
    FastBackToBackEnable = 1 << 9,
    InterruptDisable = 1 << 10,
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
    pub fn to_header(&self) -> PciHeader {
        PciHeader::new(self.cfg_addr)
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

    pub fn set_memory_base(&self, base: u16) {
        self.regs().memory_base.set(base);
    }

    pub fn set_memory_limit(&self, limit: u16){
        self.regs().memory_limit.set(limit);
    }
}
