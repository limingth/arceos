use core::mem::MaybeUninit;

pub mod host_controllers;

pub enum MightBeInited<T> {
    Inited(T),
    Uninit(MaybeUninit<T>),
}

impl<T> Default for MightBeInited<T> {
    fn default() -> Self {
        Self::Uninit(MaybeUninit::zeroed())
    }
}
