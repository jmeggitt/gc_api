use crate::inner::{MarkCompactAlloc, MarkWord};
use gc_api::alloc::Alloc;
use gc_api::mark::Mark;
use gc_api::trace::{Trace, Tracer, TracingAllocator};
use gc_api::Gc;
use std::mem::size_of;

pub struct MarkCompactTracer<'a> {
    gc: &'a MarkCompactAlloc,
    mark_state: bool,
    pub traced: usize,
}

impl<'a> MarkCompactTracer<'a> {
    pub(crate) fn new(gc: &'a MarkCompactAlloc, mark_state: bool) -> Self {
        MarkCompactTracer {
            gc,
            mark_state,
            traced: 0,
        }
    }
}

impl TracingAllocator for MarkCompactAlloc {
    type Tracer<'a> = MarkCompactTracer<'a>;
}

impl<'a> Tracer<'a, MarkCompactAlloc> for MarkCompactTracer<'a> {
    fn trace_obj<T>(&mut self, obj: &Gc<T, MarkCompactAlloc>)
    where
        T: ?Sized + Trace<MarkCompactAlloc>,
        MarkCompactAlloc: Alloc<T>,
    {
        unsafe {
            let ptr = <MarkCompactAlloc as Alloc<T>>::handle_ptr(self.gc, obj.as_raw()).as_ptr();
            let mark_ptr = (ptr as usize - size_of::<MarkWord>()) as *mut MarkWord;

            // Ensure this object is marked and return early if it is.
            if (*mark_ptr).swap_mark_state(self.mark_state) == self.mark_state {
                return;
            }
        }

        self.traced += 1;
        unsafe {
            <MarkCompactAlloc as Alloc<T>>::handle_ref(self.gc, obj.as_raw()).trace(self);
        }
    }
}
