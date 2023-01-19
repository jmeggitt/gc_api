use crate::trace::roots::{GcRootStorage, RootStorage};
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

impl<'r, R, A, S> From<&'r mut R> for StackRoots<'r, R, A, S>
    where
        R: RootStorage<A>,
        S: Array<Item = R::Index>,
{
    fn from(root_source: &'r mut R) -> Self {
        StackRoots {
            storage: SmallVec::new(),
            root_source,
            _phantom: PhantomData,
        }
    }
}

impl<'r, R, A, S> RootStorage<A> for StackRoots<'r, R, A, S>
where
    R: RootStorage<A>,
    S: Array<Item = R::Index>,
{
    type Index = R::Index;

    #[inline(always)]
    fn remove_root(&mut self, index: Self::Index) -> bool {
        self.root_source.remove_root(index)
    }
}

impl<'r, T, R, A, S> GcRootStorage<T, A> for StackRoots<'r, R, A, S>
where
    A: Alloc<T>,
    R: RootStorage<A> + GcRootStorage<T, A>,
    S: Array<Item = R::Index>,
{
    #[inline(always)]
    fn add_root(&mut self, root: &Gc<T, A>) -> Self::Index {
        self.root_source.add_root(root)
    }
}

impl<'r, R, S, A> Drop for StackRoots<'r, R, A, S>
where
    S: Array<Item = R::Index>,
    R: RootStorage<A>,
{
    fn drop(&mut self) {
        while let Some(root) = self.storage.pop() {
            self.root_source.remove_root(root);
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

#[cfg(feature = "slab")]
use crate::trace::{roots::UntypedTracable, Trace, TracingAllocator};

#[cfg(feature = "slab")]
pub type UniformHandleRoots<A, R> = slab::Slab<UntypedTracable<A, R>>;

#[cfg(feature = "slab")]
impl<A: TracingAllocator, R> RootStorage<A> for UniformHandleRoots<A, R> {
    type Index = usize;

    #[inline(always)]
    fn remove_root(&mut self, index: Self::Index) -> bool {
        self.try_remove(index).is_some()
    }
}

#[cfg(feature = "slab")]
impl<T, A, R> GcRootStorage<T, A> for UniformHandleRoots<A, R>
where
    T: Trace<A>,
    A: Alloc<T, RawHandle = R> + TracingAllocator,
    R: Clone,
{
    #[inline(always)]
    fn add_root(&mut self, root: &Gc<T, A>) -> Self::Index {
        self.insert(UntypedTracable::from_handle(root.clone()))
    }
}
