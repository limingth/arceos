use core::{fmt::Debug, mem::MaybeUninit};

pub mod host_controllers;

#[derive(Clone, Debug)]
pub enum MightBeInited<T>
where
    T: Clone,
{
    Inited(T),
    Uninit,
}

impl<T> Default for MightBeInited<T>
where
    T: Clone + Debug,
{
    fn default() -> Self {
        Self::Uninit
    }
}
