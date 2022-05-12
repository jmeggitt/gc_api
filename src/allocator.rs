use crate::Gc;
use std::alloc::Layout;
use std::ptr::NonNull;

pub trait RawAllocator {
    type Error;

    fn try_alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, Self::Error>;
}

/// An allocator which can be used as a handle to mutate the heap.
pub trait Allocator {
    type Error;

    /// Allocate an object. Potentially performing a savepoint to do so.
    fn alloc<T>(&mut self, x: T) -> Gc<T, Self>
    where
        Self: Alloc<T>;

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    fn try_alloc<T>(&mut self) -> Result<Gc<T, Self>, Self::Error>
    where
        Self: Alloc<T>;

    /// Mark a location where this allocator can safely yield to garbage collection. Garbage
    /// connection will only occur when the required allocators have yielded. Calling this function
    /// does not guarantee garbage collection will occur.
    fn yield_point(&mut self);
}

/// A marker trait which can be used to indicate a type can be allocated by an allocator.
pub trait Alloc<T: ?Sized>: Sized {
    type RawHandle: Sized;
}

// pub trait ObjectMark<T: ?Sized>: Alloc<T> {
//     type MarkType: Sized;
//
//     unsafe fn mark(handle: &Self::RawHandle) -> &Self::MarkType;
// }
