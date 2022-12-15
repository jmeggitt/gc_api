use crate::trace::MarkSweepTracer;
use std::alloc::Layout;
use std::cell::RefCell;
use gc_api::alloc::{Accessor, Alloc};
use gc_api::error::Error;
use std::ptr::NonNull;
use gc_api::trace::Trace;
use log::{trace, debug};

mod layout;
mod mark;
mod heap;
mod reference_table;


pub use mark::MarkWord;
use crate::inner::heap::MarkSweepImpl;
pub use layout::{Object, ObjectHandle};

#[repr(transparent)]
pub struct MarkSweepAlloc(MarkSweepImpl);
// pub struct MarkSweepAlloc(NonNull<MarkSweepImpl>);

impl MarkSweepAlloc {
    // pub fn with_capacity(capacity: usize) -> Self {
    //     let gc_impl = MarkSweepImpl::with_capacity(capacity);
    //     let ptr = NonNull::new(Box::into_raw(Box::new(gc_impl)));
    //
    //     MarkSweepAlloc(ptr.expect("Box can not produce null"))
    // }
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

            let mut tracer = MarkSweepTracer::new( self, self.0.global_mark_state);
            trace!("Tracing shared roots");
            roots.trace(&mut tracer);
            trace!("Found a total of {} objects", tracer.traced);

        unsafe {
            let bytes_cleared = self.0.perform_sweep();
            debug!("Performed cleanup which cleared {} bytes of space",bytes_cleared);

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

    unsafe fn access<'g>(&'g self, handle: &'g <MarkSweepAlloc as Alloc<T>>::RawHandle) -> Result<Self::Guard<'g>, Error> {
        Ok(&*handle.as_ptr().read().cast().as_ptr())
    }
}




