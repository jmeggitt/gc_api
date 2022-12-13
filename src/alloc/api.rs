use std::alloc::Layout;
use std::fmt::Debug;
use std::ptr;
use std::ptr::NonNull;

use crate::error::Error;
use crate::error::ErrorKind::OutOfMemory;
use crate::{Alloc, AllocMut, Gc, GcMut};

pub const DEFAULT_ALLOC_RETRY_LIMIT: Option<u32> = Some(3);

/// Notes:
/// - Encode metadata in Gc pointer
/// - Swap error out with trait system?
pub trait Allocator {
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

    #[inline(always)]
    fn alloc<T>(&mut self, val: T) -> Gc<T, Self>
    where
        Self: Alloc<T>,
    {
        self.alloc_with(|| val)
    }

    #[inline(always)]
    fn alloc_mut<T>(&mut self, val: T) -> GcMut<T, Self>
    where
        Self: AllocMut<T>,
        <Self as Alloc<T>>::MutTy: From<T>,
    {
        self.alloc_with::<_, <Self as Alloc<T>>::MutTy>(|| val.into())
    }

    #[inline(always)]
    fn try_alloc<T>(&mut self, val: T) -> Result<Gc<T, Self>, Error>
    where
        Self: Alloc<T>,
    {
        self.try_alloc_with(|| val)
    }

    #[inline(always)]
    fn alloc_with<F, T>(&mut self, f: F) -> Gc<T, Self>
    where
        F: FnOnce() -> T,
        Self: Alloc<T>,
    {
        self.try_gc_alloc_with(DEFAULT_ALLOC_RETRY_LIMIT, f)
            .unwrap_or_else(|err| failed_allocation(err))
    }

    #[inline(always)]
    fn try_gc_alloc_with<F, T>(
        &mut self,
        retry_limit: Option<u32>,
        f: F,
    ) -> Result<Gc<T, Self>, Error>
    where
        F: FnOnce() -> T,
        Self: Alloc<T>,
    {
        let layout = Layout::new::<T>();

        unsafe {
            self.try_gc_alloc_init(retry_limit, layout, |ptr| {
                ptr::write(ptr.as_ptr() as *mut T, f())
            })
        }
    }

    /// This function attempts to allocate a new object on the heap in accordance to the given
    /// layout. The caller can then choose how they would like to initialize that memory.
    ///
    /// # Safety
    /// The caller must fully initialize the object data via the init function. Failing to do so may
    /// result in undefined behavior comparable to calling [`std::mem::MaybeUninit::assume_init`]
    /// without fully initializing the type.
    #[inline(always)]
    unsafe fn try_gc_alloc_init<F, T>(
        &mut self,
        mut retry_limit: Option<u32>,
        layout: Layout,
        init: F,
    ) -> Result<Gc<T, Self>, Error>
    where
        T: ?Sized,
        F: FnOnce(NonNull<u8>),
        Self: Alloc<T>,
    {
        let handle = loop {
            match unsafe { Alloc::<T>::try_alloc_layout(self, layout) } {
                Ok(handle) => break handle,
                Err(err) if err.kind() == OutOfMemory => {
                    // Decrement retry counter
                    match &mut retry_limit {
                        None => {}
                        Some(0) => return Err(err),
                        Some(x) => *x -= 1,
                    }

                    // Request that a GC be performed and attempt to yield
                    self.request_gc(CollectionType::AllocAtLeast(layout));
                    self.yield_point();
                }
                Err(err) => return Err(err),
            };
        };

        unsafe {
            let data_ptr = Alloc::<T>::handle_ptr(self, &handle);
            debug_assert!(
                data_ptr.as_ptr() as usize & (layout.align() - 1) == 0,
                "GC allocation did not meet required alignment"
            );

            init(data_ptr);
            Ok(Gc::from_raw(handle))
        }
    }

    /// This function is intended to be a safe equivalent for [`try_gc_alloc_init`]. To avoid any
    /// unsafe code the unitialized value is first initialized to its default value before being
    /// handed to the user.
    #[inline(always)]
    fn try_gc_alloc_setup<F, T>(
        &mut self,
        retry_limit: Option<u32>,
        init: F,
    ) -> Result<Gc<T, Self>, Error>
    where
        T: ?Sized + Default,
        F: FnOnce(&mut T),
        Self: Alloc<T>,
    {
        let layout = Layout::new::<T>();

        unsafe {
            self.try_gc_alloc_init(retry_limit, layout, |ptr| {
                let mut_ref = &mut *ptr.cast().as_ptr();
                // Hopefully the compiler will understand that this call can be optimized away
                *mut_ref = T::default();
                init(mut_ref);
            })
        }
    }

    #[inline(always)]
    fn try_alloc_with<F, T>(&mut self, f: F) -> Result<Gc<T, Self>, Error>
    where
        F: FnOnce() -> T,
        Self: Alloc<T>,
    {
        self.try_gc_alloc_with(None, f)
    }

    #[inline(always)]
    fn alloc_slice_copy<T>(&mut self, src: &[T]) -> Gc<[T], Self>
    where
        T: Copy,
        Self: Alloc<[T]>,
    {
        let layout = Layout::new::<T>();

        unsafe {
            self.try_gc_alloc_init(DEFAULT_ALLOC_RETRY_LIMIT, layout, |ptr| {
                ptr::copy_nonoverlapping(src.as_ptr(), ptr.as_ptr() as *mut T, src.len());
            })
            .unwrap_or_else(|err| failed_allocation(err))
        }
    }

    #[inline(always)]
    fn alloc_slice_clone<T>(&mut self, src: &[T]) -> Gc<[T], Self>
    where
        T: Clone,
        Self: Alloc<[T]>,
    {
        self.alloc_slice_fill_with(src.len(), |index| src[index].clone())
    }

    #[inline(always)]
    fn alloc_str(&mut self, src: &str) -> Gc<str, Self>
    where
        Self: Alloc<str>,
    {
        let layout = Layout::for_value(src.as_bytes());

        unsafe {
            self.try_gc_alloc_init(DEFAULT_ALLOC_RETRY_LIMIT, layout, |ptr| {
                ptr::copy_nonoverlapping(src.as_ptr(), ptr.as_ptr(), src.len());
            })
            .unwrap_or_else(|err| failed_allocation(err))
        }
    }

    #[inline(always)]
    fn alloc_slice_fill_with<T, F>(&mut self, len: usize, f: F) -> Gc<[T], Self>
    where
        F: FnMut(usize) -> T,
        Self: Alloc<[T]>,
    {
        self.try_alloc_slice_fill_with(len, f)
            .unwrap_or_else(|err| failed_allocation(err))
    }

    #[inline(always)]
    fn try_alloc_slice_fill_with<T, F>(&mut self, len: usize, f: F) -> Result<Gc<[T], Self>, Error>
    where
        F: FnMut(usize) -> T,
        Self: Alloc<[T]>,
    {
        self.try_gc_alloc_slice_fill_with(Some(0), len, f)
    }

    #[inline(always)]
    fn try_gc_alloc_slice_fill_with<T, F>(
        &mut self,
        retry_limit: Option<u32>,
        len: usize,
        mut f: F,
    ) -> Result<Gc<[T], Self>, Error>
    where
        F: FnMut(usize) -> T,
        Self: Alloc<[T]>,
    {
        let layout = Layout::array::<T>(len).unwrap_or_else(|err| failed_allocation(err));

        unsafe {
            self.try_gc_alloc_init(retry_limit, layout, |ptr| {
                for index in 0..len {
                    ptr::write(ptr.cast::<T>().as_ptr().add(index), f(index));
                }
            })
        }
    }

    #[inline(always)]
    fn alloc_slice_fill_copy<T>(&mut self, len: usize, value: T) -> Gc<[T], Self>
    where
        T: Copy,
        Self: Alloc<[T]>,
    {
        self.alloc_slice_fill_with(len, |_| value)
    }

    #[inline(always)]
    fn alloc_slice_fill_clone<T>(&mut self, len: usize, value: &T) -> Gc<[T], Self>
    where
        T: Clone,
        Self: Alloc<[T]>,
    {
        self.alloc_slice_fill_with(len, |_| value.clone())
    }

    #[inline(always)]
    fn alloc_slice_fill_iter<T, I>(&mut self, iter: I) -> Gc<[T], Self>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        Self: Alloc<[T]>,
    {
        let mut iter = iter.into_iter();

        self.alloc_slice_fill_with(iter.len(), |_| {
            iter.next().expect("Iterator supplied too few elements")
        })
    }

    #[inline(always)]
    fn alloc_slice_fill_default<T>(&mut self, len: usize) -> Gc<[T], Self>
    where
        T: Default,
        Self: Alloc<[T]>,
    {
        self.alloc_slice_fill_with(len, |_| T::default())
    }
}

pub enum CollectionType {
    /// Request that a full GC is performed across the entire heap as soon as possible
    Full,
    /// Request that a partial GC is performed. The definition of a partial GC will vary slightly
    /// between different implementations.
    Partial,
    /// Request a GC to be performed so the current allocator has enough space to allocate the given
    /// layout. What this means will change depending on the implementation.
    AllocAtLeast(Layout),
    /// Suggest to the GC that garbage collection should be performed soon. This is designed to
    /// give a user the option to hint that space will be required in the near future. The
    /// implementation may choose to ignore this request if it deems a GC is not required or would
    /// be detrimental to performance.
    Suggest,
    /// A custom collection request defined by a specific implementation.
    Custom(u64),
}


#[cold]
#[inline(never)]
fn failed_allocation<T: Debug>(err: T) -> ! {
    panic!("Failed to perform GC allocation: {:?}", err)
}
