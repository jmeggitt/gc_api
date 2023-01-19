use crate::trace::MarkCompactTracer;
use gc_api::alloc::{Accessor, AccessorMut, Alloc};
use gc_api::error::Error;
use gc_api::trace::Trace;
use log::{debug, trace};
use std::alloc::Layout;
use std::cell::RefCell;
use std::ptr::NonNull;

mod heap;
mod layout;
mod mark;
mod reference_table;

use crate::inner::heap::MarkCompactImpl;
pub use layout::{Object, ObjectHandle};
pub use mark::MarkWord;

#[repr(transparent)]
pub struct MarkCompactAlloc(MarkCompactImpl);

impl MarkCompactAlloc {
    pub fn with_capacity(capacity: usize) -> Self {
        let gc_impl = MarkCompactImpl::with_capacity(capacity);

        MarkCompactAlloc(gc_impl)
    }

    #[inline(always)]
    pub fn should_perform_gc(&mut self) -> bool {
        self.0.requested_gc
    }

    #[cold]
    #[inline(never)]
    pub fn perform_gc<T: Trace<Self>>(&mut self, roots: &T) -> usize {
        debug!("Performing GC");
        {
            let MarkCompactAlloc(inner) = self;

            inner.requested_gc = false;
            inner.global_mark_state = !inner.global_mark_state;
        }

        let mut tracer = MarkCompactTracer::new(self, self.0.global_mark_state);
        trace!("Tracing shared roots");
        roots.trace(&mut tracer);
        trace!("Found a total of {} objects", tracer.traced);

        unsafe {
            let bytes_cleared = self.0.perform_compact();
            debug!(
                "Performed cleanup which cleared {} bytes of space",
                bytes_cleared
            );

            bytes_cleared
        }
    }

    pub fn gc_at_next_yield(&mut self) {
        let MarkCompactAlloc(inner) = self;

        inner.requested_gc = true;
    }
}

impl<T: Sized> Alloc<T> for MarkCompactAlloc {
    type MutTy = RefCell<T>;
    type RawHandle = NonNull<NonNull<Object>>;

    type Flags = Self;

    unsafe fn try_alloc_layout(&mut self, layout: Layout) -> Result<Self::RawHandle, Error> {
        let MarkCompactAlloc(inner) = self;

        inner.alloc(layout)
    }

    unsafe fn handle_ptr(&self, handle: &<Self as Alloc<T>>::RawHandle) -> NonNull<u8> {
        handle.as_ptr().read().cast()
    }

    unsafe fn handle_ref(&self, handle: &<Self as Alloc<T>>::RawHandle) -> &T {
        <Self as Alloc<T>>::handle_ptr(self, handle).cast().as_ref()
    }
}

pub struct MarkCompactAccessor;

impl<T: 'static> Accessor<T, MarkCompactAlloc> for MarkCompactAccessor {
    type Guard<'g> = &'g T;

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <MarkCompactAlloc as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error> {
        Ok(&*handle.as_ptr().read().cast().as_ptr())
    }
}

impl<T: 'static> AccessorMut<T, MarkCompactAlloc> for MarkCompactAccessor {
    type GuardMut<'g> = std::cell::RefMut<'g, T>;

    unsafe fn access_mut<'g>(
        &'g self,
        handle: &'g <MarkCompactAlloc as Alloc<<MarkCompactAlloc as Alloc<T>>::MutTy>>::RawHandle,
    ) -> Result<Self::GuardMut<'g>, Error> {
        Ok((*handle
            .as_ptr()
            .read()
            .cast::<<MarkCompactAlloc as Alloc<T>>::MutTy>()
            .as_ptr())
        .borrow_mut())
    }
}
