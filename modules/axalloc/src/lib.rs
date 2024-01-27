//! [ArceOS](https://github.com/rcore-os/arceos) global memory allocator.
//!
//! It provides [`GlobalAllocator`], which implements the trait
//! [`core::alloc::GlobalAlloc`]. A static global variable of type
//! [`GlobalAllocator`] is defined with the `#[global_allocator]` attribute, to
//! be registered as the standard library’s default allocator.

#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

mod page;

use allocator::{AllocResult, BaseAllocator, BitmapPageAllocator, ByteAllocator, PageAllocator};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use spinlock::SpinNoIrq;

const PAGE_SIZE: usize = 0x1000;
const MIN_HEAP_SIZE: usize = 0x8000; // 32 K

pub use page::GlobalPage;

cfg_if::cfg_if! {
    if #[cfg(feature = "slab")] {
        use allocator::SlabByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "buddy")] {
        use allocator::BuddyByteAllocator as DefaultByteAllocator;
    } else if #[cfg(feature = "tlsf")] {
        use allocator::TlsfByteAllocator as DefaultByteAllocator;
    }
}

/// The global allocator used by ArceOS.
///
/// It combines a [`ByteAllocator`] and a [`PageAllocator`] into a simple
/// two-level allocator: firstly tries allocate from the byte allocator, if
/// there is no memory, asks the page allocator for more memory and adds it to
/// the byte allocator.
///
/// Currently, [`TlsfByteAllocator`] is used as the byte allocator, while
/// [`BitmapPageAllocator`] is used as the page allocator.
///
/// [`TlsfByteAllocator`]: allocator::TlsfByteAllocator
pub struct GlobalAllocator {
    balloc_free: SpinNoIrq<DefaultByteAllocator>,
    balloc_nocache: SpinNoIrq<DefaultByteAllocator>,
    palloc_free: SpinNoIrq<BitmapPageAllocator<PAGE_SIZE>>,
}

impl GlobalAllocator {
    /// Creates an empty [`GlobalAllocator`].
    pub const fn new() -> Self {
        Self {
            balloc_free: SpinNoIrq::new(DefaultByteAllocator::new()),
            balloc_nocache: SpinNoIrq::new(DefaultByteAllocator::new()),
            palloc_free: SpinNoIrq::new(BitmapPageAllocator::new()),
        }
    }

    /// Returns the name of the allocator.
    pub const fn name(&self) -> &'static str {
        cfg_if::cfg_if! {
            if #[cfg(feature = "slab")] {
                "slab"
            } else if #[cfg(feature = "buddy")] {
                "buddy"
            } else if #[cfg(feature = "tlsf")] {
                "TLSF"
            }
        }
    }

    /// Initializes the allocator with the given region.
    ///
    /// It firstly adds the whole region to the page allocator, then allocates
    /// a small region (32 KB) to initialize the byte allocator. Therefore,
    /// the given region must be larger than 32 KB.
    /// added nocache allocator-2024.1.23
    pub fn init(
        &self,
        (free_base, free_size): (usize, usize),
        (nocache_base, nocache_size): (usize, usize),
    ) {
        {
            assert!(free_size > MIN_HEAP_SIZE);
            let init_heap_size = MIN_HEAP_SIZE;
            self.palloc_free.lock().init(free_base, free_size);
            let heap_ptr = self
                .alloc_pages(init_heap_size / PAGE_SIZE, PAGE_SIZE)
                .unwrap();
            self.balloc_free.lock().init(heap_ptr, init_heap_size);
        }

        if nocache_size > 0 {
            self.balloc_nocache.lock().init(nocache_base, nocache_size);
        }
    }

    /// Add the given region to the allocator.
    ///
    /// It will add the whole region to the byte allocator.
    pub fn add_free_memory(&self, start_vaddr: usize, size: usize) -> AllocResult {
        self.balloc_free.lock().add_memory(start_vaddr, size)
    }

    /// Add the given region to the allocator.
    ///
    /// It will add the whole region to the byte allocator.
    pub fn add_nocache_memory(&self, start_vaddr: usize, size: usize) -> AllocResult {
        let mut g = self.balloc_nocache.lock();
        if g.total_bytes()==0 {
            return  Err(allocator::AllocError::NoMemory);
        }
        g.add_memory(start_vaddr, size)
    } //TODO 大脑爆炸

    /// Allocate arbitrary number of bytes. Returns the left bound of the
    /// allocated region.
    ///
    /// It firstly tries to allocate from the byte allocator. If there is no
    /// memory, it asks the page allocator for more memory and adds it to the
    /// byte allocator.
    ///
    /// `align_pow2` must be a power of 2, and the returned region bound will be
    ///  aligned to it.
    pub fn alloc(&self, layout: Layout) -> AllocResult<NonNull<u8>> {
        // simple two-level allocator: if no heap memory, allocate from the page allocator.
        let mut balloc = self.balloc_free.lock();
        loop {
            if let Ok(ptr) = balloc.alloc(layout) {
                return Ok(ptr);
            } else {
                let old_size = balloc.total_bytes();
                let expand_size = old_size
                    .max(layout.size())
                    .next_power_of_two()
                    .max(PAGE_SIZE);
                let heap_ptr = self.alloc_pages(expand_size / PAGE_SIZE, PAGE_SIZE)?;
                debug!(
                    "expand heap memory: [{:#x}, {:#x})",
                    heap_ptr,
                    heap_ptr + expand_size
                );
                balloc.add_memory(heap_ptr, expand_size)?;
            }
        }
    }

    /// Gives back the allocated region to the byte allocator.
    ///
    /// The region should be allocated by [`alloc`], and `align_pow2` should be
    /// the same as the one used in [`alloc`]. Otherwise, the behavior is
    /// undefined.
    ///
    /// [`alloc`]: GlobalAllocator::alloc
    pub fn dealloc(&self, pos: NonNull<u8>, layout: Layout) {
        self.balloc_free.lock().dealloc(pos, layout)
    }

    /// Allocate arbitrary number of bytes. Returns the left bound of the
    /// allocated region.
    ///
    /// It firstly tries to allocate from the byte allocator. If there is no
    /// memory, it asks the page allocator for more memory and adds it to the
    /// byte allocator.
    ///
    /// `align_pow2` must be a power of 2, and the returned region bound will be
    ///  aligned to it.
    pub fn alloc_nocache(&self, layout: Layout) -> AllocResult<NonNull<u8>> {
        // simple two-level allocator: if no heap memory, allocate from the page allocator.
        let mut balloc = self.balloc_nocache.lock();
        balloc.alloc(layout)
    }

    /// Gives back the allocated region to the byte allocator.
    ///
    /// The region should be allocated by [`alloc`], and `align_pow2` should be
    /// the same as the one used in [`alloc`]. Otherwise, the behavior is
    /// undefined.
    ///
    /// [`alloc`]: GlobalAllocator::alloc
    pub fn dealloc_nocache(&self, pos: NonNull<u8>, layout: Layout) {
        self.balloc_nocache.lock().dealloc(pos, layout)
    }

    /// Allocates contiguous pages.
    ///
    /// It allocates `num_pages` pages from the page allocator.
    ///
    /// `align_pow2` must be a power of 2, and the returned region bound will be
    /// aligned to it.
    pub fn alloc_pages(&self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        self.palloc_free.lock().alloc_pages(num_pages, align_pow2)
    }

    /// Gives back the allocated pages starts from `pos` to the page allocator.
    ///
    /// The pages should be allocated by [`alloc_pages`], and `align_pow2`
    /// should be the same as the one used in [`alloc_pages`]. Otherwise, the
    /// behavior is undefined.
    ///
    /// [`alloc_pages`]: GlobalAllocator::alloc_pages
    pub fn dealloc_pages(&self, pos: usize, num_pages: usize) {
        self.palloc_free.lock().dealloc_pages(pos, num_pages)
    }



    /// Returns the number of allocated bytes in the byte allocator.
    pub fn used_bytes(&self) -> usize {
        self.balloc_free.lock().used_bytes()
    }

    /// Returns the number of available bytes in the byte allocator.
    pub fn available_bytes(&self) -> usize {
        self.balloc_free.lock().available_bytes()
    }

    /// Returns the number of allocated pages in the page allocator.
    pub fn used_pages(&self) -> usize {
        self.palloc_free.lock().used_pages()
    }

    /// Returns the number of available pages in the page allocator.
    pub fn available_pages(&self) -> usize {
        self.palloc_free.lock().available_pages()
    }

    /// Returns the number of allocated bytes in the byte allocator.
    pub fn used_bytes_nocache(&self) -> usize {
        self.balloc_nocache.lock().used_bytes()
    }

    /// Returns the number of available bytes in the byte allocator.
    pub fn available_bytes_nocache(&self) -> usize {
        self.balloc_nocache.lock().available_bytes()
    }


}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Ok(ptr) = GlobalAllocator::alloc(self, layout) {
            ptr.as_ptr()
        } else {
            alloc::alloc::handle_alloc_error(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        GlobalAllocator::dealloc(self, NonNull::new(ptr).expect("dealloc null ptr"), layout)
    }
}

#[cfg_attr(all(target_os = "none", not(test)), global_allocator)]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

/// Returns the reference to the global allocator.
pub fn global_allocator() -> &'static GlobalAllocator {
    &GLOBAL_ALLOCATOR
}

/// Initializes the global allocator with the given memory region.
///
/// Note that the memory region bounds are just numbers, and the allocator
/// does not actually access the region. Users should ensure that the region
/// is valid and not being used by others, so that the allocated memory is also
/// valid.
///
/// This function should be called only once, and before any allocation.
pub fn global_init(free: (usize, usize), nocache: (usize, usize)) {
    debug!(
        "initialize global allocator at: free-[{:#x}, {:#x}),nocache-[{:#x},{:#x}]",
        free.0,
        free.0 + free.1,
        nocache.0,
        nocache.0 + nocache.1
    );
    GLOBAL_ALLOCATOR.init(free, nocache);
}

/// Add the given memory region to the global allocator.
///
/// Users should ensure that the region is valid and not being used by others,
/// so that the allocated memory is also valid.
///
/// It's similar to [`global_init`], but can be called multiple times.
pub fn global_add_free_memory(start_vaddr: usize, size: usize) -> AllocResult {
    debug!(
        "add a memory region to global allocator: [{:#x}, {:#x})",
        start_vaddr,
        start_vaddr + size
    );
    GLOBAL_ALLOCATOR.add_free_memory(start_vaddr, size)
}
