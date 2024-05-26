use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use core::convert::TryInto;
use spinning_top::Spinlock;
use xhci::Registers;

use crate::host::mapper::Mapper;

static REGISTERS: OnceCell<Spinlock<Registers<Mapper>>> = OnceCell::uninit();

/// # Safety
///
/// `mmio_base` must be the correct one.
pub(crate) unsafe fn init(mmio_base: VirtAddr) {
    let mmio_base: usize = mmio_base.as_usize();

    REGISTERS
        .try_init_once(|| Spinlock::new(Registers::new(mmio_base, Mapper)))
        .expect("Failed to initialize `REGISTERS`.")
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
    T: FnOnce(&mut Registers<Mapper>) -> U,
{
    let mut r = REGISTERS.try_get().unwrap().lock();
    f(&mut r)
}
