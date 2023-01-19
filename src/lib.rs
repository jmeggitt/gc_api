//! A collection of traits and structures to help define the semantics of a multithreading garbage
//! collector.
use crate::alloc::access::Accessor;
use crate::alloc::{Alloc, AllocMut};
use crate::error::Error;

pub mod alloc;
pub mod error;
pub mod mark;
pub mod trace;

/// An owned handle into a garbage collected heap. The heap should outlive
pub trait Heap {
    /// The type of handles for this heap. The primary purpose of handles is to provide guarantee
    /// the lifetime of the heap.
    type Handle;
    type Allocator;

    /// Create a new allocator into this heap. It is recommended that allocators implement `Send` so
    /// they may be shared across threads and used as thread local allocators. Similarly to heap
    /// handles, the heap is expected to outlive all allocators.
    fn create_allocator(&self) -> Self::Allocator;

    /// Create a new handle for this heap. While not required, it may be beneficial to make
    /// `type Handle = Self;`. It is assumed that handles provide shared ownership of the heap. This
    /// may be implemented via reference counting (Ex: `Arc<Self>`) or as a wrapper for unsafe code.
    fn handle(&self) -> Self::Handle;
}

/// A pointer into the heap. Depending on how the implementing garbage collector is implemented,
/// the data stored in a GC pointer can be accessed in one of a few ways.
///
/// ```rust
/// # use gc_api::Gc;
/// let mut item: Gc<i32, SomeAllocator> = allocator.alloc(3);
///
/// // `Gc<T>` may implement `Deref` to mimic `Arc<T>` in function. However, this is the least safe
/// // approach to access data and may trigger a panic if unsupported by the current GC.
/// let deref: &i32 = &*item;
///
/// // This is the recommended way to access data since it provides extra safety guarantees. The
/// // reference to an allocator prevents garbage collection from being performed while the item is
/// // in use and ensures that the heap is still alive. It also has the option to use the allocator
/// // to get information necessary to dereference the item.
/// let with_alloc: &i32 = item.with(&allocator);
/// ```
#[repr(transparent)]
pub struct Gc<T: ?Sized, H: Alloc<T>> {
    handle: <H as Alloc<T>>::RawHandle,
}

impl<T: ?Sized, H: Alloc<T>> Gc<T, H> {
    /// Use an allocator to access data held within a handle.
    ///
    /// This function is syntactic sugar for `allocator.read(self)`.
    #[inline(always)]
    pub fn get<'a, A>(&'a self, allocator: &'a A) -> <A as Accessor<T, H>>::Guard<'a>
    where
        A: Accessor<T, H>,
    {
        allocator.read(self)
    }

    #[inline(always)]
    pub fn try_get<'a, A>(
        &'a self,
        allocator: &'a A,
    ) -> Result<<A as Accessor<T, H>>::Guard<'a>, Error>
    where
        A: Accessor<T, H>,
    {
        allocator.try_read(self)
    }

    /// Converts a `Gc<T>` into the underlying raw handle type.
    pub fn into_raw(self) -> <H as Alloc<T>>::RawHandle {
        self.handle
    }

    /// Get a reference into the underlying raw handle type.
    pub fn as_raw(&self) -> &<H as Alloc<T>>::RawHandle {
        &self.handle
    }

    /// Reconstructs a Gc<T> from a raw handle type.
    ///
    /// # Safety
    /// This function should only be used with an unmodified raw handle produced by [`Gc::into_raw`]
    /// or by an underlying garbage collector implementation to create a new `Gc<T>`.
    pub unsafe fn from_raw(raw: <H as Alloc<T>>::RawHandle) -> Self {
        Gc { handle: raw }
    }
}

impl<T: ?Sized, H: Alloc<T>> Copy for Gc<T, H> where <H as Alloc<T>>::RawHandle: Copy {}

impl<T: ?Sized, H: Alloc<T>> Clone for Gc<T, H>
where
    <H as Alloc<T>>::RawHandle: Clone,
{
    fn clone(&self) -> Self {
        Gc {
            handle: self.handle.clone(),
        }
    }
}

pub type GcMut<T, H> = Gc<<H as Alloc<T>>::MutTy, H>;
