/** Request status of the specific recipient */
pub const REQUEST_GET_STATUS: u8 = 0x00;

/** Clear or disable a specific feature */
pub const REQUEST_CLEAR_FEATURE: u8 = 0x01;

/* 0x02 is reserved */

/** Set or enable a specific feature */
pub const REQUEST_SET_FEATURE: u8 = 0x03;

/* 0x04 is reserved */

/** Set device address for all future accesses */
pub const REQUEST_SET_ADDRESS: u8 = 0x05;

/** Get the specified descriptor */
pub const REQUEST_GET_DESCRIPTOR: u8 = 0x06;

/** Used to update existing descriptors or add new descriptors */
pub const REQUEST_SET_DESCRIPTOR: u8 = 0x07;

/** Get the current device configuration value */
pub const REQUEST_GET_CONFIGURATION: u8 = 0x08;

/** Set device configuration */
pub const REQUEST_SET_CONFIGURATION: u8 = 0x09;

/** Return the selected alternate setting for the specified interface */
pub const REQUEST_GET_INTERFACE: u8 = 0x0a;

/** Select an alternate interface for the specified interface */
pub const REQUEST_SET_INTERFACE: u8 = 0x0b;

/** Set then report an endpoint's synchronization frame */
pub const REQUEST_SYNCH_FRAME: u8 = 0x0c;

/** Sets both the U1 and U2 Exit Latency */
pub const REQUEST_SET_SEL: u8 = 0x30;

/** Delay from the time a host transmits a packet to the time it is
 * received by the device. */
pub const SET_ISOCH_DELAY: u8 = 0x31;

/** Standard */
pub const REQUEST_TYPE_STANDARD: u8 = (0x00 << 5);

/** Class */
pub const REQUEST_TYPE_CLASS: u8 = (0x01 << 5);

/** Vendor */
pub const REQUEST_TYPE_VENDOR: u8 = (0x02 << 5);

/** Reserved */
pub const REQUEST_TYPE_RESERVED: u8 = (0x03 << 5);

/** Device */
pub const RECIPIENT_DEVICE: u8 = 0x00;

/** Interface */
pub const RECIPIENT_INTERFACE: u8 = 0x01;

/** Endpoint */
pub const RECIPIENT_ENDPOINT: u8 = 0x02;

/** Other */
pub const RECIPIENT_OTHER: u8 = 0x03;

/** Out: host-to-device */
pub const ENDPOINT_OUT: u8 = 0x00;

/** In: device-to-host */
pub const ENDPOINT_IN: u8 = 0x80;

// HID Class-Specific Requests values. See section 7.2 of the HID specifications
pub const HID_GET_REPORT: u8 = 0x01;
pub const HID_GET_IDLE: u8 = 0x02;
pub const HID_GET_PROTOCOL: u8 = 0x03;
pub const HID_SET_REPORT: u8 = 0x09;
pub const HID_SET_IDLE: u8 = 0x0A;
pub const HID_SET_PROTOCOL: u8 = 0x0B;
pub const HID_REPORT_TYPE_INPUT: u16 = 0x01;
pub const HID_REPORT_TYPE_OUTPUT: u16 = 0x02;
pub const HID_REPORT_TYPE_FEATURE: u16 = 0x03;

// Mass Storage Requests values. See section 3 of the Bulk-Only Mass Storage Class specifications
pub const BOMS_RESET: u8 = 0xFF;
pub const BOMS_GET_MAX_LUN: u8 = 0xFE;

// Section 5.2: Command Status Wrapper (CSW)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CommandStatusWrapper {
    pub csw_signature: [u8; 4],
    pub csw_tag: u32,
    pub csw_data_residue: u32,
    pub csw_status: u8,
}

// Section 5.1: Command Block Wrapper (CBW)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandBlockWrapper {
    pub cbw_signature: [u8; 4],
    pub cbw_tag: u32,
    pub cbw_data_transfer_length: u32,
    pub cbw_flags: u8,
    pub cbw_lun: u8,
    pub cbw_cb_length: u8,
    pub cbw_cb: [u8; 16],
}
pub const CDB_LENGTH: [u8; 256] = [
    //	 0  1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, //  0
    06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, 06, //  1
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, //  2
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, //  3
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, //  4
    10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, //  5
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  6
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  7
    16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, //  8
    16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, //  9
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, //  A
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, //  B
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  C
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  D
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  E
    00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, //  F
];

pub fn ep_num_to_dci(ep: usize) -> u8 {
    const MASK_IN: usize = ENDPOINT_IN as usize;
    if ep == 0 {
        1
    } else if ep & (MASK_IN) > 0 {
        ((ep - MASK_IN) * 2 + 1) as u8
    } else {
        (ep * 2) as u8
    }
}

#[cfg(test)]
mod test {
    use super::ep_num_to_dci;

    #[test]
    fn test_dci() {
        let ep = 0x81;
        let dci = ep_num_to_dci(ep);
        assert_eq!(dci, 2);
    }
}
