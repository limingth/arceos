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
