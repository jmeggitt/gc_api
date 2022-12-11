use crate::error::Error;
use std::alloc::Layout;

pub mod access;
pub mod allocator;
pub mod marker;

/// A marker trait which can be used to indicate a type can be allocated by an allocator.
pub trait Alloc<T: ?Sized>: Sized {
    type MutAlternative;
    type RawHandle: Sized;

    type Flags;

    /// Performs allocation for the specified type
    ///
    /// # Safety
    /// The given layout must be a valid layout for some variation of `T`. This applies to both
    /// `Sized` and DST `T`s.
    unsafe fn try_alloc(&mut self, layout: Layout) -> Result<Self::RawHandle, Error>;
}

/// This trait is a helper that can be added to trait bounds. Writing `A: AllocMut<T>` is equivalent
/// to `A: Alloc<T> + Alloc<<Self as Alloc<T>>::MutAlternative>`
pub trait AllocMut<T: ?Sized>: Alloc<T> + Alloc<<Self as Alloc<T>>::MutAlternative> {}

impl<T: ?Sized, A> AllocMut<T> for A where A: Alloc<T> + Alloc<<Self as Alloc<T>>::MutAlternative> {}
