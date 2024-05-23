
pub(crate) const USBDEV_MAX_FUNCTIONS: u16 = 10;
pub(crate) enum TDeviceNameSelector {
    DeviceNameVendor,
    DeviceNameDevice,
    DeviceNameUnknown,
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

pub(crate) struct USBHostController;
pub(crate) struct USBHCIRootPort;
pub(crate) struct USBStandardHub;
pub(crate) struct USBEndpoint;

pub(crate) struct USBDevice {
    //todo use xhci::context::Device64Byte;
    p_host: USBHostController,
    p_root_port: USBHCIRootPort,
    p_hub: USBStandardHub,
    p_hub_port_index: u16,
    uc_address: u8,
    //todo
}
