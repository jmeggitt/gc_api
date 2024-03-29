use crate::error::Error;
use std::alloc::Layout;
use std::ptr::NonNull;

pub mod access;
pub mod api;
pub mod marker;

pub use access::*;
pub use api::*;
pub use marker::*;

/// A marker trait which can be used to indicate a type can be allocated by an allocator.
pub trait Alloc<T: ?Sized>: Sized {
    /// An alternative to T that will guarentee that the created value can be accessed mutably by
    /// [crate::alloc::access::AccessorMut]. The purpose of having this is that some GCs will have
    /// built in locking capabilities and can avoid the need for manually using some form of lock
    /// and having this field allows for a generic approach for switching between using an extra
    /// lock and the regular value.
    ///
    /// For where `T: Sized`, it can be assumed `MutTy: From<T>`. This bound is not included since
    /// this trait covers DSTs too and it would complicate the process for this to be required.
    type MutTy;

    type RawHandle: Sized;

    type Flags;

    /// Performs allocation for the specified type
    ///
    /// # Safety
    /// The given layout must be a valid layout for some variation of `T`. This applies to both
    /// `Sized` and DST `T`s.
    unsafe fn try_alloc_layout(&mut self, layout: Layout) -> Result<Self::RawHandle, Error>;

    /// Retrieves a pointer to the memory on the heap for a given handle
    ///
    /// # Safety
    /// This function should only be used in a single thread context. To safely call this function,
    /// handle should not represent an object that may be in use on another thread. The primary use
    /// of this function is initialzing new allocations, but may also include finalizing objects
    /// before dropping.
    unsafe fn handle_ptr(&self, handle: &Self::RawHandle) -> NonNull<u8>;

    /// Unlike handle_ptr, the purpose of this function is to also provide pointer metadata (such as
    /// with slices). This can be helpful for dealing with some cases where some types are handled
    /// in a different implementation.
    ///
    /// # Safety
    /// This function can only be used when exclusive access is held.
    unsafe fn handle_ref(&self, handle: &Self::RawHandle) -> &T;
}

/// This trait is a helper that can be added to trait bounds. Writing `A: AllocMut<T>` is equivalent
/// to `A: Alloc<T> + Alloc<<Self as Alloc<T>>::MutAlternative>`
pub trait AllocMut<T: ?Sized>: Alloc<T> + Alloc<<Self as Alloc<T>>::MutTy> {
    type RawHandle: Sized;
}

impl<T: ?Sized, A> AllocMut<T> for A
where
    A: Alloc<T> + Alloc<<Self as Alloc<T>>::MutTy>,
{
    type RawHandle = <A as Alloc<<A as Alloc<T>>::MutTy>>::RawHandle;
}
