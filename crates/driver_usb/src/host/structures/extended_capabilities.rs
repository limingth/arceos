// SPDX-License-Identifier: GPL-3.0-or-later

use {
    super::registers,
    crate::host::xhci::MemoryMapper,
    axhal::mem::PhysAddr,
    conquer_once::spin::OnceCell,
    core::convert::TryInto,
    spinning_top::Spinlock,
    xhci::{extended_capabilities, ExtendedCapability},
};

static EXTENDED_CAPABILITIES: OnceCell<
    Spinlock<Option<extended_capabilities::List<MemoryMapper>>>,
> = OnceCell::uninit();

pub(crate) unsafe fn init(mmio_base: usize) {
    let hccparams1 = registers::handle(|r| r.capability.hccparams1.read_volatile());

    EXTENDED_CAPABILITIES
        .try_init_once(|| unsafe {
            Spinlock::new({
                let list = extended_capabilities::List::new(
                    (mmio_base as u64).try_into().unwrap(),
                    hccparams1,
                    MemoryMapper,
                );
                list
            })
        })
        .expect("Failed to initialize `EXTENDED_CAPABILITIES`.");
}

pub(crate) fn iter() -> Option<
    impl Iterator<
        Item = Result<ExtendedCapability<MemoryMapper>, extended_capabilities::NotSupportedId>,
    >,
> {
    Some(
        EXTENDED_CAPABILITIES
            .try_get()
            .expect("`EXTENDED_CAPABILITIES` is not initialized.`")
            .lock()
            .as_mut()?
            .into_iter(),
    )
}
