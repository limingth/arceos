// SPDX-License-Identifier: GPL-3.0-or-later

use {
    crate::host::xhci::MemoryMapper, axhal::mem::PhysAddr, conquer_once::spin::OnceCell,
    core::convert::TryInto, spinning_top::Spinlock, xhci::Registers,
};

static REGISTERS: OnceCell<Spinlock<Registers<MemoryMapper>>> = OnceCell::uninit();

pub(crate) unsafe fn init(mmio_base: usize) {
    REGISTERS
        .try_init_once(|| Spinlock::new(unsafe { Registers::new(mmio_base, MemoryMapper {}) }))
        .expect("Failed to initialize `REGISTERS`.");
}

/// Handle xHCI registers.
///
/// To avoid deadlocking, this method takes a closure. Caller is supposed not to call this method
/// inside the closure, otherwise a deadlock will happen.
///
/// Alternative implementation is to define a method which returns `impl Deref<Target =
/// Registers>`, but this will expand the scope of the mutex guard, increasing the possibility of
/// deadlocks.
pub(crate) fn handle<T, U>(f: T) -> U
where
    T: FnOnce(&mut Registers<MemoryMapper>) -> U,
{
    let mut r = REGISTERS.try_get().unwrap().lock();
    f(&mut r)
}
