use crate::inner::{MarkSweepAlloc, MarkWord};
use gc_api::alloc::Alloc;
use gc_api::mark::Mark;
use gc_api::trace::{Trace, Tracer, TracingAllocator};
use gc_api::Gc;
use std::mem::size_of;

pub struct MarkSweepTracer<'a> {
    gc: &'a MarkSweepAlloc,
    mark_state: bool,
    pub traced: usize,
}

impl<'a> MarkSweepTracer<'a> {
    pub(crate) fn new(gc: &'a MarkSweepAlloc, mark_state: bool) -> Self {
        MarkSweepTracer {
            gc,
            mark_state,
            traced: 0,
        }
    }
}

impl TracingAllocator for MarkSweepAlloc {
    type Tracer<'a> = MarkSweepTracer<'a>;
}

impl<'a> Tracer<'a, MarkSweepAlloc> for MarkSweepTracer<'a> {
    fn trace_obj<T>(&mut self, obj: &Gc<T, MarkSweepAlloc>)
    where
        T: ?Sized + Trace<MarkSweepAlloc>,
        MarkSweepAlloc: Alloc<T>,
    {
        unsafe {
            let ptr = <MarkSweepAlloc as Alloc<T>>::handle_ptr(self.gc, obj.as_raw()).as_ptr();
            let mark_ptr = (ptr as usize - size_of::<MarkWord>()) as *mut MarkWord;

            // Ensure this object is marked and return early if it is.
            if (*mark_ptr).swap_mark_state(self.mark_state) == self.mark_state {
                return;
            }
        }

        self.traced += 1;
        unsafe {
            <MarkSweepAlloc as Alloc<T>>::handle_ref(self.gc, obj.as_raw()).trace(self);
        }
    }
}
