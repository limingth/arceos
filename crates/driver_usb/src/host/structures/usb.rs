// usb.rs

#[allow(dead_code)] // Enable if needed, to allow unused code
pub(crate) mod usb {
    use alloc::vec::Vec;

    #[repr(u8)]
    pub enum TUSBPID {  //usb pid
        Setup = 0,
        Data0 = 1,
        Data1 = 2,
        // USBPIDData2,
        // USBPIDMData
    }

    pub const USB_DEFAULT_ADDRESS: u8 = 0;
    pub const USB_FIRST_DEDICATED_ADDRESS: u8 = 1;
    pub const USB_MAX_ADDRESS: u8 = 63;

    #[repr(u8)]
    pub enum TUSBSpeed {
        Low = 0,
        Full,
        High,
        Super,
        Unknown,
    }

    #[repr(u8)]
    pub enum TUSBError {
        Stall = 0,
        Transaction,
        Babble,
        FrameOverrun,
        DataToggle,
        HostBus,
        Split,
        Timeout,
        Aborted,
        Unknown,
    }

    #[derive(PartialEq, Eq, Debug)]
    pub struct TSetupData {
        pub bm_request_type: u8,
        pub b_request: u8,
        pub w_value: u16,
        pub w_index: u16,
        pub w_length: u16,
        //Data follows
    }

    pub const REQUEST_OUT: u8 = 0;
    pub const REQUEST_IN: u8 = 0x80;

    pub const REQUEST_CLASS: u8 = 0x20;
    pub const REQUEST_VENDOR: u8 = 0x40;

    pub const REQUEST_TO_DEVICE: u8 = 0;
    pub const REQUEST_TO_INTERFACE: u8 = 1;
    pub const REQUEST_TO_ENDPOINT: u8 = 2;
    pub const REQUEST_TO_OTHER: u8 = 3;

    pub const GET_STATUS: u8 = 0;
    pub const CLEAR_FEATURE: u8 = 1;
    pub const SET_FEATURE: u8 = 3;
    pub const SET_ADDRESS: u8 = 5;
    pub const GET_DESCRIPTOR: u8 = 6;
    pub const SET_CONFIGURATION: u8 = 9;
    pub const SET_INTERFACE: u8 = 11;

    pub const ENDPOINT_HALT: u8 = 0;

    pub const DESCRIPTOR_DEVICE: u8 = 1;
    pub const DESCRIPTOR_CONFIGURATION: u8 = 2;
    pub const DESCRIPTOR_STRING: u8 = 3;
    pub const DESCRIPTOR_INTERFACE: u8 = 4;
    pub const DESCRIPTOR_ENDPOINT: u8 = 5;

    pub const DESCRIPTOR_CS_INTERFACE: u8 = 36;
    pub const DESCRIPTOR_CS_ENDPOINT: u8 = 37;
    pub const DESCRIPTOR_INDEX_DEFAULT: u8 = 0;

    #[derive(Debug, Default)]
    pub struct TUSBDeviceDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub bcd_usb: u16,
        pub b_device_class: u8,
        pub b_device_sub_class: u8,
        pub b_device_protocol: u8,
        pub b_max_packet_size0: u8,
        pub id_vendor: u16,
        pub id_product: u16,
        pub bcd_device: u16,
        pub i_manufacturer: u8,
        pub i_product: u8,
        pub i_serial_number: u8,
        pub b_num_configurations: u8,

    }
    impl TUSBDeviceDescriptor {
        pub const USB_DEFAULT_MAX_PACKET_SIZE: u8 = 0;
    }

    #[derive(Debug, Default)]
    pub struct TUSBConfigurationDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub w_total_length: u16,
        pub b_num_interfaces: u8,
        pub b_configuration_value: u8,
        pub i_configuration: u8,
        pub bm_attributes: u8,
        pub b_max_power: u8,
    }

    #[derive(Debug, Default)]
    pub struct TUSBInterfaceDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_interface_number: u8,
        pub b_alternate_setting: u8,
        pub b_num_endpoints: u8,
        pub b_interface_class: u8,
        pub b_interface_sub_class: u8,
        pub b_interface_protocol: u8,
        pub i_interface: u8,
    }

    #[derive(Debug, Default)]
    pub struct TUSBEndpointDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_endpoint_address: u8,
        pub bm_attributes: u8,
        pub w_max_packet_size: u16,
        pub b_interval: u8,
    }

    // Union types are not directly supported in Rust, so we'll represent it as an enum
    #[derive(Debug)]
    pub enum TUSBDescriptor {
        Header { b_length: u8, b_descriptor_type: u8 },
        Configuration(TUSBConfigurationDescriptor),
        Interface(TUSBInterfaceDescriptor),
        Endpoint(TUSBEndpointDescriptor),
        //TODO
        //AudioEndpoint(TUSBAudioEndpointDescriptor), // Add the corresponding struct if available
        //MIDIStreamingEndpoint(TUSBMIDIStreamingEndpointDescriptor), // Add the corresponding struct if available
    }

    #[derive(Debug, Default)]
    pub struct TUSBStringDescriptor {
        pub b_length: u8,
        pub b_descriptor_type: u8,
        pub b_string: Vec<u16>,
    }
}
