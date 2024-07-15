use core::num::NonZeroUsize;

#[derive(Clone, Copy, Debug)]
pub struct Mapper;
impl xhci::accessor::Mapper for Mapper {
    unsafe fn map(&mut self, physical_address: usize, _: usize) -> NonZeroUsize {
        NonZeroUsize::new(physical_address).expect("physical_address is zero")
    }

    fn unmap(&mut self, _: usize, _: usize) {}
}
