use aarch64_cpu::asm::barrier;
use axhal::mem::phys_to_virt;
use core::num::NonZeroUsize;
use xhci::accessor::Mapper;

#[cfg(feature = "phytium")]
pub mod phytium;
#[cfg(feature = "vl805")]
pub mod vl805;
mod xhci_operations;

const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;

#[derive(Clone, Copy)]
pub struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}
