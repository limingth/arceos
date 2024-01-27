use core::{
    alloc::{Allocator, Layout},
    mem::size_of,
    ops::{Deref, DerefMut},
    ptr::{slice_from_raw_parts, NonNull},
};

use log::debug;

pub struct DMAVec<'a, A: Allocator, T> {
    layout: Layout,
    ptr: NonNull<[T]>,
    allocator: &'a A,
}

impl<'a, A: Allocator, T> DMAVec<'a, A, T> {
    pub fn new(size: usize, align: usize, allocator: &'a A) -> Self {
        let buff_size = size * size_of::<T>();
        let layout = Layout::from_size_align(buff_size, align).unwrap();
        let buff = allocator.allocate(layout).unwrap();
        let ptr;
        unsafe {
            let s = &*slice_from_raw_parts(buff.as_ptr() as *const T, size);
            ptr = NonNull::from(s);
        }
        Self {
            layout,
            ptr,
            allocator,
        }
    }
}

impl<'a, A: Allocator, T> Deref for DMAVec<'a, A, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}
impl<'a, A: Allocator, T> DerefMut for DMAVec<'a, A, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<'a, A: Allocator, T> Drop for DMAVec<'a, A, T> {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.ptr.cast::<u8>();
            self.allocator.deallocate(ptr, self.layout);
        }
    }
}
