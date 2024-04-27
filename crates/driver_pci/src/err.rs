use core::fmt::{Display, Formatter};

/// Errors accessing a PCI device.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PciError {
    /// The device reported an invalid BAR type.
    InvalidBarType,
}

impl Display for PciError {
    fn fmt(&self, f: &mut Formatter) -> core::fmt::Result {
        match self {
            Self::InvalidBarType => write!(f, "Invalid PCI BAR type."),
        }
    }
}

pub type Result<T = ()> = core::result::Result<T, PciError>;
