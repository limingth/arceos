use core::num::NonZeroUsize;
use xhci::accessor::Mapper;
use crate::addr::VirtAddr;


#[derive(Clone)]
pub struct MemMapper;
impl Mapper for MemMapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
        return NonZeroUsize::new_unchecked(phys_start);
    }
    fn unmap(&mut self, virt_start: usize, bytes: usize) {}
}

pub type Registers = xhci::Registers<MemMapper>;


pub fn new_registers(mmio_base: VirtAddr)->Registers{
    unsafe { xhci::Registers::new(mmio_base.as_usize(), MemMapper) }
}