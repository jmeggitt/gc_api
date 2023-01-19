use crate::inner::{MarkCompactAlloc, ObjectHandle};
use crate::trace::MarkCompactTracer;
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

pub struct MarkCompactGC {
    alloc: MarkCompactAlloc,
    roots: UniformHandleRoots<MarkCompactAlloc, ObjectHandle>,
}

impl MarkCompactGC {
    /// Create a new mark and compact GC with the given heap size.
    pub fn with_capacity(len: usize) -> Self {
        MarkCompactGC {
            alloc: MarkCompactAlloc::with_capacity(len),
            roots: Default::default(),
        }
    }
}

impl<T: 'static> Accessor<T, MarkCompactAlloc> for MarkCompactGC {
    type Guard<'g> = &'g T;

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <MarkCompactAlloc as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error> {
        Ok(&*handle.as_ptr().read().cast().as_ptr())
    }
}

impl Allocator for MarkCompactGC {
    type Alloc = MarkCompactAlloc;
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

impl Trace<MarkCompactAlloc> for MarkCompactGC {
    fn trace(&self, tracer: &mut MarkCompactTracer) {
        self.roots.trace(tracer);
    }
}

impl RootStorage<MarkCompactAlloc> for MarkCompactGC {
    type Index = usize;

    fn remove_root(&mut self, index: Self::Index) -> bool {
        self.roots.remove_root(index)
    }
}

impl<T> GcRootStorage<T, MarkCompactAlloc> for MarkCompactGC
where
    T: Trace<MarkCompactAlloc>,
    MarkCompactAlloc: Alloc<T, RawHandle = ObjectHandle>,
{
    fn add_root(&mut self, root: &Gc<T, MarkCompactAlloc>) -> Self::Index {
        self.roots.add_root(root)
    }
}
