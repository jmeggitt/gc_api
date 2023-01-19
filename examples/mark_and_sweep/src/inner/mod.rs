use crate::trace::MarkSweepTracer;
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

use crate::inner::heap::MarkSweepImpl;
pub use layout::{Object, ObjectHandle};
pub use mark::MarkWord;

#[repr(transparent)]
pub struct MarkSweepAlloc(MarkSweepImpl);

impl MarkSweepAlloc {
    pub fn with_capacity(capacity: usize) -> Self {
        let gc_impl = MarkSweepImpl::with_capacity(capacity);

        MarkSweepAlloc(gc_impl)
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
            let MarkSweepAlloc(inner) = self;

            inner.requested_gc = false;
            inner.global_mark_state = !inner.global_mark_state;
        }

        let mut tracer = MarkSweepTracer::new(self, self.0.global_mark_state);
        trace!("Tracing shared roots");
        roots.trace(&mut tracer);
        trace!("Found a total of {} objects", tracer.traced);

        unsafe {
            let bytes_cleared = self.0.perform_sweep();
            debug!(
                "Performed cleanup which cleared {} bytes of space",
                bytes_cleared
            );

            bytes_cleared
        }
    }

    pub fn gc_at_next_yield(&mut self) {
        let MarkSweepAlloc(inner) = self;

        inner.requested_gc = true;
    }
}

impl<T: Sized> Alloc<T> for MarkSweepAlloc {
    type MutTy = RefCell<T>;
    type RawHandle = NonNull<NonNull<Object>>;

    type Flags = Self;

    unsafe fn try_alloc_layout(&mut self, layout: Layout) -> Result<Self::RawHandle, Error> {
        let MarkSweepAlloc(inner) = self;

        inner.alloc(layout)
    }

    unsafe fn handle_ptr(&self, handle: &<Self as Alloc<T>>::RawHandle) -> NonNull<u8> {
        handle.as_ptr().read().cast()
    }

    unsafe fn handle_ref(&self, handle: &<Self as Alloc<T>>::RawHandle) -> &T {
        <Self as Alloc<T>>::handle_ptr(self, handle).cast().as_ref()
    }
}

pub struct MarkSweepAccessor;

impl<T: 'static> Accessor<T, MarkSweepAlloc> for MarkSweepAccessor {
    type Guard<'g> = &'g T;

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <MarkSweepAlloc as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error> {
        Ok(&*handle.as_ptr().read().cast().as_ptr())
    }
}

impl<T: 'static> AccessorMut<T, MarkSweepAlloc> for MarkSweepAccessor {
    type GuardMut<'g> = std::cell::RefMut<'g, T>;

    unsafe fn access_mut<'g>(
        &'g self,
        handle: &'g <MarkSweepAlloc as Alloc<<MarkSweepAlloc as Alloc<T>>::MutTy>>::RawHandle,
    ) -> Result<Self::GuardMut<'g>, Error> {
        Ok((*handle
            .as_ptr()
            .read()
            .cast::<<MarkSweepAlloc as Alloc<T>>::MutTy>()
            .as_ptr())
        .borrow_mut())
    }
}
