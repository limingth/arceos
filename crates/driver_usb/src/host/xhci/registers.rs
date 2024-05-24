use crate::addr::VirtAddr;
use core::num::NonZeroUsize;
use log::debug;
use xhci::{accessor::Mapper, extended_capabilities::XhciSupportedProtocol, ExtendedCapability};

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
pub struct Registers {
    pub regs: RegistersBase,
    pub ext_list: Option<RegistersExtList>,
}

impl Registers {
    pub fn new_registers(mmio_base: VirtAddr) -> Self {
        unsafe {
            let regs = RegistersBase::new(mmio_base.as_usize(), MemMapper);
            let ext_list = RegistersExtList::new(
                mmio_base.as_usize(),
                regs.capability.hccparams1.read(),
                MemMapper,
            );

            Self { regs, ext_list }
        }
    }

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
}
