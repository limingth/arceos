pub(super) mod extended_capabilities;
pub(super) mod registers;
pub(super) mod roothub;
pub(super) mod xhci_command_manager;
pub(super) mod xhci_device;
pub(super) mod xhci_event_manager;
pub(super) mod xhci_slot_manager;

pub(crate) mod command_ring;
pub(crate) mod event_ring;
pub(super) mod scratchpad;

const XHCI_CONFIG_EVENT_RING_SIZE: usize = 256;
const XHCI_TRB_CONTROL_C: usize = 1 << 0;
const XHCI_LINK_TRB_CONTROL_TC: usize = 1 << 1;
const XHCI_TRB_CONTROL_TRB_TYPE_SHIFT: usize = 10;
const XHCI_EVENT_TRB_STATUS_COMPLETION_CODE_SHIFT: usize = 24;
const XHCI_TRANSFER_EVENT_TRB_STATUS_TRB_TRANSFER_LENGTH_MASK: usize = 0xFFFFFF;
const XHCI_CMD_COMPLETION_EVENT_TRB_CONTROL_SLOTID_SHIFT: usize = 24;
const XHCI_TRANSFER_EVENT_TRB_CONTROL_ENDPOINTID_MASK: usize = 0x1F << 16;
const XHCI_TRANSFER_EVENT_TRB_CONTROL_ENDPOINTID_SHIFT: usize = 16;
const XHCI_PORT_STATUS_EVENT_TRB_PARAMETER1_PORTID_SHIFT: usize = 24;
const XHCI_CONFIG_IMODI: u16 = 500;
const XHCI_CONFIG_MAX_PORTS : usize = 5;
const XHCI_CONFIG_MAX_SLOTS: usize = 64;
const DMA_ADDRESS: usize = 0xfd50_0000; //TODO sus
                                        //TODO FIX VIRTUAL ADDRESS
