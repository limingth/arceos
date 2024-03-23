pub(super) mod extended_capabilities;
pub(super) mod registers;
pub(super) mod roothub;
pub(super) mod xhci_command_manager;
pub(super) mod xhci_event_manager;
pub(super) mod xhci_slot_manager;

pub(crate) mod command_ring;
pub(crate) mod event_ring;
pub(super) mod scratchpad;

const XHCI_CONFIG_EVENT_RING_SIZE: usize = 256;
const XHCI_TRB_CONTROL_C: usize = 1 << 0;
const XHCI_LINK_TRB_CONTROL_TC: usize = 1 << 1;
const XHCI_TRB_CONTROL_TRB_TYPE_SHIFT: usize = 10;
const XHCI_CONFIG_IMODI: u16 = 500;
const XHCI_CONFIG_MAX_SLOTS: usize = 64;
//const DMA_ADDRESS: usize = 0xfd50_0000; //TODO sus
//TODO FIX VIRTUAL ADDRESS
