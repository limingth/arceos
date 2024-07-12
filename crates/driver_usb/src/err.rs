use alloc::string::String;
use core::fmt::Display;
use xhci::ring::trb::event::CompletionCode;

#[derive(Debug)]
pub enum Error {
    Unknown(String),
    Param(String),
    CMD(CompletionCode),
    TimeOut,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Unknown(msg) => write!(f, "unknown usb err: {}", msg),
            Error::Param(msg) => write!(f, "param err: {}", msg),
            Error::TimeOut => write!(f, "timeout"),
            Error::CMD(cmd) => write!(f, "cmd fail: {:#?}", cmd),
        }
    }
}

pub type Result<T = ()> = core::result::Result<T, Error>;
