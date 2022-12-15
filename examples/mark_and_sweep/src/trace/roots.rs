use gc_api::alloc::Alloc;
use gc_api::mark::Mark;
use gc_api::root::RootSource;
use gc_api::trace::{Trace, Tracer, TracingAllocator};
use gc_api::{Gc, Heap};
use log::debug;
use std::mem::size_of;
use crate::inner::{MarkSweepAlloc, MarkWord, ObjectHandle};

#[derive(Default)]
pub struct RootList {
    inner: Vec<RootEntry>,
}

struct RootEntry {
    ptr: ObjectHandle,
    trace_fn: fn(ObjectHandle, &mut MarkSweepTracer),
}

impl Trace<MarkSweepAlloc> for RootList {
    fn trace(&self, tracer: &mut MarkSweepTracer) {
        debug!("Performing trace of {} roots", self.inner.len());
        for entry in &self.inner {
            (entry.trace_fn)(entry.ptr, tracer);
        }
    }
}

pub struct RootIndex {
    root_ptr: ObjectHandle,
    index: usize,
}

impl RootSource<MarkSweepAlloc> for RootList {
    type Index = RootIndex;

    fn add_root<T>(&mut self, root: &Gc<T, MarkSweepAlloc>) -> Self::Index
    where
        T: Trace<MarkSweepAlloc>,
    {
        fn trace_fn<K>(ptr: ObjectHandle, tracer: &mut MarkSweepTracer)
        where
            K: Trace<MarkSweepAlloc>,
        {
            let handle: Gc<K, MarkSweepAlloc> = unsafe { Gc::from_raw(ptr) };
            handle.trace(tracer);
        }

        self.inner.push(RootEntry {
            ptr: root.into_raw(),
            trace_fn: trace_fn::<T>,
        });

        RootIndex {
            root_ptr: root.into_raw(),
            index: self.inner.len(),
        }
    }

    fn remove_root<T>(&mut self, root: &Gc<T, MarkSweepAlloc>) -> bool
    where
        T: Trace<MarkSweepAlloc>,
    {
        self.remove_by_index(RootIndex {
            root_ptr: root.into_raw(),
            index: self.inner.len(),
        })
    }

    fn remove_by_index(&mut self, index: Self::Index) -> bool {
        for idx in (0..index.index.min(self.inner.len())).rev() {
            if self.inner[idx].ptr == index.root_ptr {
                self.inner.swap_remove(idx);
                return true;
            }
        }

        false
    }
}

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
            let ptr = <MarkSweepAlloc as Alloc<T>>::handle_ptr(&self.gc, obj.as_raw()).as_ptr();
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
