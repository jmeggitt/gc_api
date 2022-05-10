//! A collection of traits and structures to help define the semantics of a multithreading garbage
//! collector.
use std::ops::Deref;
use crate::allocator::Alloc;

pub mod mark;
pub mod allocator;
pub mod trace;
pub mod pointer;


/// An owned handle into a garbage collected heap.
pub trait Heap {
    type Handle;
    type Allocator;

    fn create_allocator(&self) -> Self::Allocator;

    fn handle(&self) -> Self::Handle;
}

pub struct Gc<T: ?Sized, H: Alloc<T>> {
    handle: <H as Alloc<T>>::RawHandle,
}

impl<T: ?Sized, H: Alloc<T>> Gc<T, H> {
    /// Guard is not necessary since immutable reference to allocator prevents garbage collection
    pub fn with<'a>(&'a self, allocator: &'a H) -> &'a T {
        allocator.try_allocator_deref(&self.handle)
            .expect("Attempted to deref a GC pointer which has already been dropped")
    }
}

impl<T: ?Sized, H: Alloc<T>> Deref for Gc<T, H> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        H::try_deref(&self.handle)
            .expect("Attempted to deref a GC pointer which has already been dropped")
    }
}

/// A source for gc roots which can be iterated over to .
pub trait RootSource<T> {
    type RootIter: IntoIterator<Item=T>;

    fn add_root(&mut self, root: T);
    fn remove_root(&mut self, root: T);

    fn iter_roots(&self) -> Self::RootIter;
}



