use alloc::boxed::Box;
use context::{DeviceContextList, ScratchpadBufferArray};
use core::{mem::MaybeUninit, num::NonZeroUsize};
use event::EventRing;
use log::debug;
use ring::Ring;
use spinlock::SpinNoIrq;
use xhci::{
    accessor::Mapper, extended_capabilities::XhciSupportedProtocol,
    ring::trb::event::HostController, ExtendedCapability,
};

use crate::{
    abstractions::PlatformAbstractions, host::data_structures::MightBeInited, USBSystemConfig,
};

use super::Controller;

mod context;
mod event;
mod ring;

pub type RegistersBase = xhci::Registers<MemMapper>;
pub type RegistersExtList = xhci::extended_capabilities::List<MemMapper>;
pub type SupportedProtocol = XhciSupportedProtocol<MemMapper>;
#[derive(Clone)]
pub struct MemMapper;
impl Mapper for MemMapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
        return NonZeroUsize::new_unchecked(phys_start);
    }
    fn unmap(&mut self, virt_start: usize, bytes: usize) {}
}
pub struct XHCIRegisters<O>
where
    O: PlatformAbstractions,
{
    pub regs: RegistersBase,
    pub ext_list: Option<RegistersExtList>,
    pub xhci_ctx: MightBeInited<XhciManagedContext<O>>,
}

pub struct XhciManagedContext<O>
where
    O: PlatformAbstractions,
{
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    scratchpad_buf_arr: Option<ScratchpadBufferArray<O>>,
    cmd: Ring<O>,
    event: EventRing<O>,
    pub dev_ctx: DeviceContextList<O>,
}

impl<O> XHCIRegisters<O>
where
    O: PlatformAbstractions,
{
    pub fn supported_protocol(&mut self, port: usize) -> Option<SupportedProtocol> {
        debug!("[XHCI] Find port {} protocol", port);

        if let Some(ext_list) = &mut self.ext_list {
            ext_list
                .into_iter()
                .filter_map(|one| {
                    if let Ok(ExtendedCapability::XhciSupportedProtocol(protcol)) = one {
                        return Some(protcol);
                    }
                    None
                })
                .find(|p| {
                    let head = p.header.read_volatile();
                    let port_range = head.compatible_port_offset() as usize
                        ..head.compatible_port_count() as usize;
                    port_range.contains(&port)
                })
        } else {
            None
        }
    }
    fn init(&mut self) {
        self.xhci_ctx = match self.xhci_ctx {
            MightBeInited::Inited(_) => panic!("do not init same controller twice!"),
            MightBeInited::Uninit(_) => MightBeInited::Inited(XhciManagedContext {
                max_slots: todo!(),
                max_ports: todo!(),
                max_irqs: todo!(),
                scratchpad_buf_arr: todo!(),
                cmd: todo!(),
                event: todo!(),
                dev_ctx: todo!(),
            }),
        }
    }
}

impl<O> Controller<O> for XHCIRegisters<O>
where
    O: PlatformAbstractions,
{
    fn new(config: USBSystemConfig<O>) -> Self
    where
        Self: Sized,
    {
        let mmio_base = config.base_addr.into();
        unsafe {
            let regs = RegistersBase::new(mmio_base, MemMapper);
            let ext_list =
                RegistersExtList::new(mmio_base, regs.capability.hccparams1.read(), MemMapper);
            Self {
                regs,
                ext_list,
                xhci_ctx: MightBeInited::default(),
            }
        }
    }

    fn init(&mut self) {
        self.init();
    }

    fn probe(&mut self) {
        todo!()
    }
}
