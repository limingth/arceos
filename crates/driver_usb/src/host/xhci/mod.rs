#[cfg(feature = "vl805")]
pub mod vl805;
pub mod regs;
pub mod arm_mailbox;
pub mod propertytags;
use axhal::mem::{phys_to_virt, VirtAddr, PhysAddr};
use log::info;
use xhci::accessor::Mapper;
use core::{num::NonZeroUsize, alloc::Layout, ptr::NonNull};


#[derive(Clone, Copy)]
struct MemoryMapper;

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}