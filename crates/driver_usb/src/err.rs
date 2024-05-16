use core::fmt::Display;
use alloc::string::String;

#[derive(Debug)]
pub enum Error{
    Unknown(String),
    Param(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unknown(msg) => write!(f, "unknown usb err: {}", msg),
            Error::Param(msg) => write!(f, "param err: {}", msg),
        }
    }
}



pub type Result<T=()> = core::result::Result<T, Error>;

