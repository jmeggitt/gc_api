use crate::trace::{Trace, TracingAllocator};
use crate::trace::roots::{RootStorage, GcRootStorage};
use crate::Alloc;
use crate::Gc;
use smallvec::{Array, SmallVec};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct StackRoots<
    'r,
    R: RootStorage<A>,
    A,
    S: Array<Item = R::Index> = [<R as RootStorage<A>>::Index; 8],
> {
    storage: SmallVec<S>,
    root_source: &'r mut R,
    _phantom: PhantomData<A>,
}

impl<'r, R, A, S> RootStorage<A> for StackRoots<'r, R, A, S>
    where
        R: RootStorage<A>,
        S: Array<Item = R::Index>,
{
    type Index = R::Index;

    #[inline(always)]
    fn remove_by_index(&mut self, index: Self::Index) -> bool {
        self.root_source.remove_by_index(index)
    }
}

impl<'r, T, R, A, S> GcRootStorage<T, A> for StackRoots<'r, R, A, S>
    where
        A: Alloc<T>,
        R: RootStorage<A> + GcRootStorage<T, A>,
        S: Array<Item = R::Index>,
{
    fn add_root(&mut self, root: &Gc<T, A>) -> Self::Index {
        self.root_source.add_root(root)
    }

    fn remove_root(&mut self, root: &Gc<T, A>) -> bool {
        self.root_source.remove_root(root)
    }
}

impl<'r, R, S, A> Drop for StackRoots<'r, R, A, S>
    where
        S: Array<Item = R::Index>,
        R: RootStorage<A>,
{
    fn drop(&mut self) {
        while let Some(root) = self.storage.pop() {
            self.root_source.remove_by_index(root);
        }
    }
}

impl<'r, R: RootStorage<A>, A, S: Array<Item = R::Index>> Deref for StackRoots<'r, R, A, S> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.root_source
    }
}

impl<'r, R: RootStorage<A>, A, S: Array<Item = R::Index>> DerefMut for StackRoots<'r, R, A, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.root_source
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
    fn remove_by_index(&mut self, index: Self::Index) -> bool {
        self.roots.remove_by_index(index)
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

    #[inline(always)]
    fn remove_root(&mut self, root: &Gc<T, H>) -> bool {
        self.roots.remove_root(root)
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
