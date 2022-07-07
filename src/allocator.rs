use crate::error::Error;
use crate::Gc;
use std::alloc::Layout;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

/// An allocator which can be used as a handle to mutate the heap.
pub trait Allocator {
    /// Allocate an object. Potentially performing a savepoint to do so.
    #[inline]
    fn alloc<T>(&mut self, x: T) -> Gc<T, Self>
    where
        Self: Alloc<T> + ReinterpretHandle<T>,
    {
        todo!()
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    #[inline]
    fn try_alloc<T: ?Sized>(&mut self) -> Result<Gc<T, Self>, Error>
    where
        Self: Alloc<T>,
    {
        unsafe {
            let handle = <Self as Alloc<T>>::try_alloc(self, Layout::new::<T>())?;
            Ok(Gc::from_raw(handle))
        }
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    #[inline]
    fn try_alloc_uninit<T>(&mut self) -> Result<Gc<MaybeUninit<T>, Self>, Error>
    where
        Self: Alloc<MaybeUninit<T>>,
    {
        let layout = Layout::new::<MaybeUninit<T>>();
        unsafe {
            let handle = <Self as Alloc<MaybeUninit<T>>>::try_alloc(self, layout)?;
            Ok(Gc::from_raw(handle))
        }
    }

    /// Mark a location where this allocator can safely yield to garbage collection. Garbage
    /// connection will only occur when the required allocators have yielded. Calling this function
    /// does not guarantee garbage collection will occur.
    ///
    /// For garbage collectors that do not require yield points, this will be treated as a no-op.
    fn yield_point(&mut self);

    /// Request that garbage collection is performed at the next `yield_point`. This function should
    /// only be called if recommended for the underlying implementation. Extraneous calls to this
    /// function may have an adverse effect on performance.
    ///
    /// Calling this function gives a GC a strong suggestion that garbage collection should be
    /// performed at the next opportunity. However, this does not guarantee that garbage collection
    /// can or will be performed.
    fn request_gc(&mut self);
}

/// A marker trait which can be used to indicate a type can be allocated by an allocator.
pub trait Alloc<T: ?Sized>: Sized {
    type RawHandle: Sized;

    // TODO: Is Layout necessary if T is already known?
    unsafe fn try_alloc(&mut self, layout: Layout) -> Result<Self::RawHandle, Error>;
}

/// A helper trait for transmuting a GC pointer.
///
/// TODO: May require allocator
pub trait ReinterpretHandle<T: ?Sized>: Alloc<T> {
    unsafe fn reinterpret_handle<R: ?Sized>(
        handle: <Self as Alloc<T>>::RawHandle,
    ) -> <Self as Alloc<R>>::RawHandle
    where
        Self: Alloc<R>;
}
