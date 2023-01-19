use crate::inner::{MarkSweepAlloc, ObjectHandle};
use crate::trace::MarkSweepTracer;
use gc_api::alloc::{Accessor, Alloc, Allocator, CollectionType};
use gc_api::error::Error;
use gc_api::trace::roots::{GcRootStorage, RootStorage, UniformHandleRoots};
use gc_api::trace::Trace;
use gc_api::Gc;
use log::trace;

mod inner;
mod trace;

#[cfg(test)]
mod tests;

pub struct MarkSweepGC {
    alloc: MarkSweepAlloc,
    roots: UniformHandleRoots<MarkSweepAlloc, ObjectHandle>,
}

impl MarkSweepGC {
    /// Create a new mark and sweep GC with the given heap size.
    pub fn with_capacity(len: usize) -> Self {
        MarkSweepGC {
            alloc: MarkSweepAlloc::with_capacity(len),
            roots: Default::default(),
        }
    }
}

impl<T: 'static> Accessor<T, MarkSweepAlloc> for MarkSweepGC {
    type Guard<'g> = &'g T;

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <MarkSweepAlloc as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error> {
        Ok(&*handle.as_ptr().read().cast().as_ptr())
    }
}

impl Allocator for MarkSweepGC {
    type Alloc = MarkSweepAlloc;
    fn as_raw_allocator(&mut self) -> &mut Self::Alloc {
        &mut self.alloc
    }

    fn yield_point(&mut self) {
        if self.alloc.should_perform_gc() {
            self.alloc.perform_gc(&self.roots);
        }
    }

    fn request_gc(&mut self, _collect: CollectionType) {
        trace!("Received request for GC: {:?}", _collect);
        self.alloc.gc_at_next_yield();
    }
}

impl Trace<MarkSweepAlloc> for MarkSweepGC {
    fn trace(&self, tracer: &mut MarkSweepTracer) {
        self.roots.trace(tracer);
    }
}

impl RootStorage<MarkSweepAlloc> for MarkSweepGC {
    type Index = usize;

    fn remove_root(&mut self, index: Self::Index) -> bool {
        self.roots.remove_root(index)
    }
}

impl<T> GcRootStorage<T, MarkSweepAlloc> for MarkSweepGC
where
    T: Trace<MarkSweepAlloc>,
    MarkSweepAlloc: Alloc<T, RawHandle = ObjectHandle>,
{
    fn add_root(&mut self, root: &Gc<T, MarkSweepAlloc>) -> Self::Index {
        self.roots.add_root(root)
    }
}
