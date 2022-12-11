use crate::alloc::marker::BlindTransmute;
use crate::alloc::{Alloc, AllocMut};
use crate::error::{Error, ErrorKind};
use crate::{Gc, GcMut, RootSource};
use std::alloc::Layout;

/// An allocator which can be used as a handle to mutate the heap.
pub trait Allocator: RootSource<Self> {
    /// Allocate an object. Potentially performing a savepoint to do so.
    #[inline]
    fn alloc<T>(&mut self, x: T) -> Gc<T, Self>
    where
        Self: Alloc<T>,
    {
        // unsafe {
        //     // Since we write through an immutable reference to uninitialized memory, both
        //     // `UnsafeCell<MaybeUninit<T>>` are necessary to hold up both invariants.
        //     let layout = Layout::new::<UnsafeCell<MaybeUninit<T>>>();
        //     let handle = <Self as Alloc<UnsafeCell<MaybeUninit<T>>>>::try_alloc(self, layout)?;
        //
        //     self.access(&handle)
        //
        // }

        todo!()
    }

    /// Allocate an object. Potentially performing a savepoint to do so. Unlike `Allocator::alloc`,
    /// the returned object is guarenteed to be give mutable access to `T`.
    #[inline]
    fn alloc_mut<T>(&mut self, x: T) -> GcMut<T, Self>
    where
        Self: AllocMut<T>,
    {
        todo!()
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    #[inline]
    fn try_alloc<T>(&mut self, x: T) -> Result<Gc<T, Self>, Error>
    where
        Self: Alloc<T>,
    {
        let handle = self.try_alloc_uninit()?;

        Ok(handle)
        // unsafe {
        //     let handle = <Self as Alloc<T>>::try_alloc(self, Layout::new::<T>())?;
        //     Ok(Gc::from_raw(handle))
        // }
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    ///
    /// # Safety
    /// The layout must be a valid layout for the type `T`.
    #[inline]
    unsafe fn try_alloc_dst<T: ?Sized>(&mut self, layout: Layout) -> Result<Gc<T, Self>, Error>
    where
        Self: Alloc<T>,
    {
        let handle = <Self as Alloc<T>>::try_alloc(self, layout)?;
        Ok(Gc::from_raw(handle))
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    #[inline]
    fn try_alloc_custom(&mut self, layout: Layout) -> Result<Gc<CustomAlloc, Self>, Error>
    where
        Self: Alloc<CustomAlloc>,
    {
        unsafe { self.try_alloc_dst(layout) }
    }

    /// Attempt to allocate an object. If allocation fails, an error will be returned.
    #[inline]
    fn try_alloc_uninit<T>(&mut self) -> Result<Gc<T, Self>, Error>
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
    fn alloc_uninit<T>(&mut self) -> Gc<T, Self>
    where
        Self: Alloc<T>,
    {
        let mut attempts: u32 = 0;

        loop {
            // Attempt allocation
            match self.try_alloc_uninit() {
                Ok(v) => return v,
                Err(e) if e.kind() == ErrorKind::OutOfMemory => {
                    attempts = attempts.saturating_add(1)
                }
                Err(e) => panic!("Error occured during GC alloc_uninit: {:?}", e),
            }

            // Request GC if failed
            match attempts {
                1 => self.request_gc(CollectionType::RequestMinAlloc(Layout::new::<T>())),
                2 => self.request_gc(CollectionType::RequestPartial),
                _ => self.request_gc(CollectionType::RequestFull),
            }

            // Attempt to yield to GC
            self.yield_point();
        }
    }

    /// Mark a location where this allocator can safely yield to garbage collection. Garbage
    /// connection will only occur when the required allocators have yielded. Calling this function
    /// does not guarantee garbage collection will occur.
    ///
    /// For garbage collectors that do not require yield points, this will be treated as a no-op.
    fn yield_point(&mut self);

    /// Request that garbage collection is performed at the next `yield_point`. This function should
    /// only be called if recommended by the underlying implementation. Extraneous calls to this
    /// function may have an adverse effect on performance.
    ///
    /// Calling this function gives a GC a strong suggestion that garbage collection should be
    /// performed at the next opportunity. However, this does not guarantee that garbage collection
    /// can or will be performed.
    fn request_gc(&mut self, request: CollectionType);
}

pub enum CollectionType {
    /// Request that a full GC is performed across the entire heap as soon as possible
    RequestFull,
    /// Request that a partial GC is performed. The definition of a partial GC will vary slightly
    /// between different implementations.
    RequestPartial,
    /// Request a GC to be performed so the current allocator has enough space to allocate the given
    /// layout. What this means will change depending on the implementation.
    RequestMinAlloc(Layout),
    /// Suggest to the GC that garbage collection should be performed soon. This is designed to
    /// give a user the option to hint that space will be required in the near future. The
    /// implementation may choose to ignore this request if it deems a GC is not required or would
    /// be detrimental to performance.
    Suggest,
    /// A custom collection request defined by a specific implementation.
    Custom(u64),
}

/// A placeholder type for a custom allocation.
#[repr(align(1))]
pub struct CustomAlloc {
    /// Array of unit to enforce that this is a DST
    _dst: [()],
}

impl CustomAlloc {
    pub fn as_ptr(&self) -> *const u8 {
        self as *const CustomAlloc as *const u8
    }

    pub fn as_mut(&mut self) -> *mut u8 {
        self as *mut CustomAlloc as *mut u8
    }
}

/// A helper trait for transmuting a GC pointer.
pub trait ReinterpretHandle<T: ?Sized, R: ?Sized>: Alloc<T> + Alloc<R> {
    unsafe fn reinterpret_handle(
        &mut self,
        handle: <Self as Alloc<T>>::RawHandle,
    ) -> <Self as Alloc<R>>::RawHandle;
}

impl<T, R, A> ReinterpretHandle<T, R> for A
where
    A: Alloc<T> + Alloc<R, RawHandle = <A as Alloc<T>>::RawHandle>,
    <A as Alloc<T>>::Flags: BlindTransmute,
{
    unsafe fn reinterpret_handle(
        &mut self,
        handle: <Self as Alloc<T>>::RawHandle,
    ) -> <Self as Alloc<R>>::RawHandle {
        handle
    }
}
