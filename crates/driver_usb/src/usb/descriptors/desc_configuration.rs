#[derive(Copy, Clone, Debug, Default)]
#[repr(C, packed)]
pub(crate) struct Configuration {
    length: u8,
    ty: u8,
    total_length: u16,
    num_interfaces: u8,
    config_val: u8,
    config_string: u8,
    attributes: u8,
    max_power: u8,
}
impl Configuration {
    pub(crate) fn config_val(&self) -> u8 {
        self.config_val
    }
    pub(crate) fn length(&self) -> u8 {
        self.length
    }
    pub(crate) fn ty(&self) -> u8 {
        self.ty
    }
    pub(crate) fn total_length(&self) -> u16 {
        self.total_length
    }
    pub(crate) fn num_interfaces(&self) -> u8 {
        self.num_interfaces
    }
    pub(crate) fn config_string(&self) -> u8 {
        self.config_string
    }
    pub(crate) fn attributes(&self) -> u8 {
        self.attributes
    }
    pub(crate) fn max_power(&self) -> u8 {
        self.max_power
    }
}
