#[cfg(feature = "vl805")]
pub mod vl805;
use alloc::boxed::Box;
use axalloc::{global_allocator, GlobalAllocator};
use axhal::mem::{phys_to_virt, PhysAddr, VirtAddr};
use core::{borrow::BorrowMut, cell::RefMut, f32::consts::E, num::NonZeroUsize, ptr::NonNull};
use log::info;
use xhci::{accessor::Mapper, extended_capabilities, ring::trb::event, Registers};

use self::{command_ring::CommandRing, event_ring::EventRing};

mod command_ring;
mod event_ring;

#[derive(Clone, Copy)]
struct MemoryMapper;

struct XhciController<'a> {
    controller: &'a mut dyn XhciBehaviors,
    event_ring: EventRing,
    // command_ring: CommandRing,
}

pub(crate) trait XhciBehaviors {
    fn addr(&self) -> usize;
    fn regs(&mut self) -> &mut Registers<MemoryMapper>;
    fn extra_features(&mut self) -> &mut Option<extended_capabilities::List<MemoryMapper>>;
}

impl Mapper for MemoryMapper {
    unsafe fn map(&mut self, phys_base: usize, bytes: usize) -> NonZeroUsize {
        let virt = phys_to_virt(phys_base.into());
        // info!("mapping: [{:x}]->[{:x}]", phys_base, virt.as_usize());
        return NonZeroUsize::new_unchecked(virt.as_usize());
    }

    fn unmap(&mut self, virt_base: usize, bytes: usize) {}
}

impl XhciController<'_> {
    pub(crate) fn new(xhci_behavior_impl: &mut dyn XhciBehaviors) -> Result<XhciController, ()> {
        let mut event_ring = EventRing::new(xhci_behavior_impl.borrow_mut().regs());

        Ok(XhciController {
            controller: xhci_behavior_impl,
            event_ring: event_ring,
            // command_ring: todo!(_),
            // TODO: 把EventRing和CommandRing加入对axalloc的支持
        })
    }
}
