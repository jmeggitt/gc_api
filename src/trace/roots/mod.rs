use crate::trace::{Trace, TracingAllocator};
use crate::Alloc;
use crate::Gc;
use std::ops::{Deref, DerefMut};

pub mod collections;
pub use collections::*;

/// A source for gc roots which can be iterated over. Root sources are assumed to be unordered and
/// may contain duplicate values.
pub trait RootStorage<A> {
    type Index;

    fn remove_root(&mut self, index: Self::Index) -> bool;
}

pub trait GcRootStorage<T, A: Alloc<T>>: RootStorage<A> {
    fn add_root(&mut self, root: &Gc<T, A>) -> Self::Index;
}

pub struct RootingAllocator<A, R> {
    pub alloc: A,
    pub roots: R,
}

impl<A, R, H> Trace<H> for RootingAllocator<A, R>
where
    H: TracingAllocator,
    R: Trace<H>,
{
    fn trace(&self, tracer: &mut H::Tracer<'_>) {
        self.roots.trace(tracer)
    }
}

impl<H, A, R: RootStorage<H>> RootStorage<H> for RootingAllocator<A, R> {
    type Index = R::Index;

    #[inline(always)]
    fn remove_root(&mut self, index: Self::Index) -> bool {
        self.roots.remove_root(index)
    }
}

impl<T, H, A, R> GcRootStorage<T, H> for RootingAllocator<A, R>
where
    H: Alloc<T>,
    R: RootStorage<H> + GcRootStorage<T, H>,
{
    #[inline(always)]
    fn add_root(&mut self, root: &Gc<T, H>) -> Self::Index {
        self.roots.add_root(root)
    }
}

impl<A, R> Deref for RootingAllocator<A, R> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.alloc
    }
}

impl<A, R> DerefMut for RootingAllocator<A, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.alloc
    }
}

pub struct UntypedTracable<A: TracingAllocator, R> {
    raw_handle: R,
    trace: fn(R, &mut A::Tracer<'_>),
}

impl<A: TracingAllocator, R: Clone> Trace<A> for UntypedTracable<A, R> {
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        (self.trace)(self.raw_handle.clone(), tracer)
    }
}

impl<A: TracingAllocator, R> UntypedTracable<A, R> {
    pub fn from_handle<T>(handle: Gc<T, A>) -> Self
    where
        A: Alloc<T, RawHandle = R>,
        T: Trace<A>,
    {
        fn trace_fn<K, B>(ptr: <B as Alloc<K>>::RawHandle, tracer: &mut B::Tracer<'_>)
        where
            B: TracingAllocator + Alloc<K>,
            K: Trace<B>,
        {
            let handle: Gc<K, B> = unsafe { Gc::from_raw(ptr) };
            handle.trace(tracer);
        }

        UntypedTracable {
            raw_handle: handle.into_raw(),
            trace: trace_fn::<T, A>,
        }
    }
}
