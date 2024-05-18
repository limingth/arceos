pub(crate) mod sub{
    pub(crate) const USBDEV_MAX_FUNCTIONS:u16 = 10;
    pub(crate) enum TDeviceNameSelector{
        DeviceNameVendor,
        DeviceNameDevice,
        DeviceNameUnknown
    }

    impl TDeviceNameSelector {
        //实现将TDeviceNameSelector枚举值转为数字
        pub fn to_num(&self) -> u8 {
            match self {
                TDeviceNameSelector::DeviceNameVendor => 0x01,
                TDeviceNameSelector::DeviceNameDevice => 0x02,
                TDeviceNameSelector::DeviceNameUnknown => 0x00,
            }
        }
    }

    pub(crate) struct CUSBHostController;
    pub(crate) struct CUSBHCIRootPort;
    pub(crate) struct CUSBStandardHub;
    pub(crate) struct CUSBEndpoint;

    pub(crate) struct CUSBDevice{
        p_host: CUSBHostController,
        p_root_port: CUSBHCIRootPort,
        p_hub: CUSBStandardHub,
        p_hub_port_index: u16,
        uc_address: u8,
        //todo
    }
}