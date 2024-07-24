pub mod dma;

use core::alloc::Allocator;

// pub trait PlatformAbstractions: Clone + Send + Sync + Sized {
//     type VirtAddr;
//     const PAGE_SIZE: usize;
//     type DMA: Allocator + Send + Sync + Clone;
//     fn dma_alloc(&self) -> Self::DMA;
//     fn force_sync_cache();
// }

pub trait PlatformAbstractions: OSAbstractions + HALAbstractions {}

pub trait OSAbstractions: Clone + Send + Sync + Sized {
    type VirtAddr: From<usize>;
    const PAGE_SIZE: usize;
    type DMA: Allocator + Send + Sync + Clone;
    fn dma_alloc(&self) -> Self::DMA;
    // fn interrupt_handler();//ADD Interrupt feature?
}
pub trait HALAbstractions: Clone + Send + Sync + Sized {
    fn force_sync_cache();
}
