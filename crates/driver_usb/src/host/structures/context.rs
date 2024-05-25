use super::registers;
use alloc::{boxed::Box, sync::Arc};
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axhal::mem::{PhysAddr, VirtAddr};
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::context::{
    Device32Byte, Device64Byte, DeviceHandler, Input32Byte, Input64Byte, InputControlHandler,
    InputHandler,
};

pub(crate) struct Context {
    pub(crate) input: PageBox<Input>,
    pub(crate) output: PageBox<Device>,
}
impl Default for Context {
    fn default() -> Self {
        let mut context = Self {
            input: Input::default().into(),
            output: Device::default().into(),
        };
        debug!("debug input: {:?}", (*context.input).dump_device_state());
        context
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Input {
    Byte64(PageBox<Input64Byte>),
    Byte32(PageBox<Input32Byte>),
}
impl Input {
    pub(crate) fn control_mut(&mut self) -> &mut dyn InputControlHandler {
        match self {
            Self::Byte32(b32) => b32.control_mut(),
            Self::Byte64(b64) => b64.control_mut(),
        }
    }

    pub(crate) fn device_mut(&mut self) -> &mut dyn DeviceHandler {
        match self {
            Self::Byte32(b32) => b32.device_mut(),
            Self::Byte64(b64) => b64.device_mut(),
        }
    }

    pub(crate) fn dump_device_state(&mut self) -> &mut xhci::context::Input<16> {
        match self {
            Self::Byte32(b32) => unimplemented!(),
            Self::Byte64(b64) => &mut (**b64),
        }
    }

    pub(crate) fn phys_addr(&self) -> PhysAddr {
        match self {
            Self::Byte32(b32) => b32.phys_addr(),
            Self::Byte64(b64) => b64.phys_addr(),
        }
    }

    pub(crate) fn virt_addr(&self) -> VirtAddr {
        match self {
            Self::Byte32(b32) => b32.virt_addr(),
            Self::Byte64(b64) => b64.virt_addr(),
        }
    }
}
impl Default for Input {
    fn default() -> Self {
        if csz() {
            Self::Byte64({
                let mut into: PageBox<Input64Byte> = Input64Byte::new_64byte().into();
                into.zeroed();
                into
            })
        } else {
            Self::Byte32(Input32Byte::default().into())
        }
    }
}

pub(crate) enum Device {
    Byte64(Box<Device64Byte>),
    Byte32(Box<Device32Byte>),
}
impl Default for Device {
    fn default() -> Self {
        if csz() {
            Self::Byte64(Device64Byte::default().into())
        } else {
            Self::Byte32(Device32Byte::default().into())
        }
    }
}

fn csz() -> bool {
    registers::handle(|r| r.capability.hccparams1.read_volatile().context_size())
}
