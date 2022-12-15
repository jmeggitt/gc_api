use crate::trace::{MarkSweepTracer, RootIndex, RootList};
use gc_api::alloc::{Accessor, Alloc, Allocator, CollectionType};
use gc_api::error::{Error, ErrorKind};
use gc_api::root::RootSource;
use gc_api::trace::Trace;
use gc_api::{Gc, Heap};
use log::{debug, info, trace};
use std::alloc::Layout;
use std::cell::RefCell;
use std::fmt::{self, Display, Formatter};
use std::ptr::NonNull;
use std::rc::Rc;
use crate::inner::MarkSweepAlloc;

mod inner;
mod trace;

#[cfg(test)]
mod tests;

pub struct MarkSweepGC {
    // heap: Rc<RefCell<MarkSweepImpl>>,
    alloc: MarkSweepAlloc,
    roots: RootList,
    // roots: Rc<RefCell<RootList>>,
}

// impl Clone for MarkSweepGC {
//     fn clone(&self) -> Self {
//         MarkSweepGC {
//             alloc: self.alloc,
//             roots: self.roots.clone(),
//         }
//     }
// }

// impl Display for MarkSweepGC {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         // let inner = self.alloc.borrow();
//
//         write!(
//             f,
//             "MarkSweepGC [Usage: {}B, Free: {}B, Capacity: {}B]",
//             inner.usage(),
//             inner.free_space(),
//             inner.capacity()
//         )
//     }
// }

impl MarkSweepGC {
    /// Create a new mark and sweep GC with the given heap size.
    pub fn with_capacity(len: usize) -> Self {
        // let inner = MarkSweepImpl::with_capacity(len);

        MarkSweepGC {
            alloc: MarkSweepAlloc::with_capacity(len),
            // heap: Rc::new(RefCell::new(inner)),
            roots: Default::default(),
        }
    }
}

// impl Heap for MarkSweepGC {
//     type Handle = Self;
//     type Allocator = Self;
//
//     fn create_allocator(&self) -> Self::Allocator {
//         self.alloc
//         // self.clone()
//     }
//
//     fn handle(&self) -> Self::Handle {
//         self.clone()
//     }
// }

// impl<T: Sized> Alloc<T> for MarkSweepGC {
//     type MutTy = std::sync::Mutex<T>;
//     type RawHandle = *mut u8;
//
//     type Flags = Self;
//
//     unsafe fn try_alloc_layout(&mut self, layout: Layout) -> Result<Self::RawHandle, Error> {
//         self.heap.borrow_mut().alloc(layout)
//     }
//
//     unsafe fn handle_ptr(&self, handle: &<Self as Alloc<T>>::RawHandle) -> NonNull<u8> {
//         NonNull::new_unchecked(*(*handle as *mut *mut u8))
//     }
//
//     unsafe fn handle_ref(&self, handle: &<Self as Alloc<T>>::RawHandle) -> &T {
//         <Self as Alloc<T>>::handle_ptr(self, handle).cast().as_ref()
//     }
// }

impl<T: 'static> Accessor<T, MarkSweepAlloc> for MarkSweepGC {
    type Guard<'g> = &'g T;

    // #[cfg(debug_assertions)]
    // unsafe fn access<'g>(
    //     &'g self,
    //     handle: &'g <MarkSweepAlloc as Alloc<T>>::RawHandle,
    // ) -> Result<Self::Guard<'g>, Error> {
    //     let inner = self.heap.borrow();
    //     if !inner.ref_table.contains_ptr(*handle) {
    //         info!(
    //             "Attempted to access pointer outside of ref table: {:p}",
    //             *handle
    //         );
    //         return Err(Error::new(
    //             ErrorKind::IllegalState,
    //             "Failed to access pointer",
    //         ));
    //     }
    //
    //     let obj_ptr = *(*handle as *const *const T);
    //
    //     if !inner.contains_ptr(*handle) {
    //         info!(
    //             "Attempted to access object outside of heap ({:p} - {:p}): {:p}",
    //             inner.start, inner.end, obj_ptr
    //         );
    //         return Err(Error::new(
    //             ErrorKind::IllegalState,
    //             "Failed to access pointer",
    //         ));
    //     }
    //
    //     Ok(&*obj_ptr)
    // }
    //
    //
    // #[cfg(not(debug_assertions))]
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
        // let mut inner = self.heap.borrow_mut();
        // if !inner.requested_gc {
        //     return;
        // }
        //
        // debug!("Performing GC");
        // inner.requested_gc = false;
        // inner.global_mark_state = !inner.global_mark_state;
        // let desired_mark_state = inner.global_mark_state;
        //
        // let mut tracer = MarkSweepTracer::new(self.clone(), desired_mark_state);
        // trace!("Tracing shared roots");
        // self.roots.borrow().trace(&mut tracer);
        // trace!("Found a total of {} objects", tracer.traced);
        //
        // let bytes_cleared = inner.perform_sweep();
        // debug!(
        //     "Performed cleanup which cleared {} bytes of space",
        //     bytes_cleared
        // );
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

impl RootSource<MarkSweepAlloc> for MarkSweepGC {
    type Index = RootIndex;

    fn add_root<T>(&mut self, root: &Gc<T, MarkSweepAlloc>) -> Self::Index
    where
        T: Trace<MarkSweepAlloc>,
        MarkSweepAlloc: Alloc<T>,
    {
        self.roots.add_root(root)
    }

    fn remove_by_index(&mut self, index: Self::Index) -> bool {
        self.roots.remove_by_index(index)
    }

    fn remove_root<T>(&mut self, root: &Gc<T, MarkSweepAlloc>) -> bool
    where
        T: Trace<MarkSweepAlloc>,
        MarkSweepAlloc: Alloc<T>,
    {
        self.roots.remove_root(root)
    }
}

