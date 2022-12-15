use crate::inner::MarkSweepImpl;
use crate::roots::{MarkSweepTracer, RootIndex, RootList};
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

mod inner;
mod ptr_arena;
mod roots;

#[derive(Clone)]
pub struct MarkSweepGC {
    heap: Rc<RefCell<MarkSweepImpl>>,
    roots: Rc<RefCell<RootList>>,
}

impl Display for MarkSweepGC {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let inner = self.heap.borrow();

        write!(
            f,
            "MarkSweepGC [Usage: {}B, Free: {}B, Capacity: {}B]",
            inner.usage(),
            inner.free_space(),
            inner.capacity()
        )
    }
}

impl MarkSweepGC {
    /// Create a new mark and sweep GC with the given heap size.
    pub fn with_capacity(len: usize) -> Self {
        let inner = MarkSweepImpl::with_capacity(len);

        MarkSweepGC {
            heap: Rc::new(RefCell::new(inner)),
            roots: Default::default(),
        }
    }
}

impl Heap for MarkSweepGC {
    type Handle = Self;
    type Allocator = Self;

    fn create_allocator(&self) -> Self::Allocator {
        self.clone()
    }

    fn handle(&self) -> Self::Handle {
        self.clone()
    }
}

impl<T: Sized> Alloc<T> for MarkSweepGC {
    type MutTy = std::sync::Mutex<T>;
    type RawHandle = *mut u8;

    type Flags = Self;

    unsafe fn try_alloc_layout(&mut self, layout: Layout) -> Result<Self::RawHandle, Error> {
        self.heap.borrow_mut().alloc(layout)
    }

    unsafe fn handle_ptr(&self, handle: &<Self as Alloc<T>>::RawHandle) -> NonNull<u8> {
        NonNull::new_unchecked(*(*handle as *mut *mut u8))
    }

    unsafe fn handle_ref(&self, handle: &<Self as Alloc<T>>::RawHandle) -> &T {
        <Self as Alloc<T>>::handle_ptr(self, handle).cast().as_ref()
    }
}

impl<T: 'static> Accessor<T, Self> for MarkSweepGC {
    type Guard<'g> = &'g T;

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <Self as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error> {
        let inner = self.heap.borrow();
        if !inner.ref_table.contains_ptr(*handle) {
            info!(
                "Attempted to access pointer outside of ref table: {:p}",
                *handle
            );
            return Err(Error::new(
                ErrorKind::IllegalState,
                "Failed to access pointer",
            ));
        }

        let obj_ptr = *(*handle as *const *const T);

        if !inner.contains_ptr(*handle) {
            info!(
                "Attempted to access object outside of heap ({:p} - {:p}): {:p}",
                inner.start, inner.end, obj_ptr
            );
            return Err(Error::new(
                ErrorKind::IllegalState,
                "Failed to access pointer",
            ));
        }

        Ok(&*obj_ptr)
    }
}

impl Allocator for MarkSweepGC {
    type Alloc = Self;
    fn as_raw_allocator(&mut self) -> &mut Self::Alloc {
        self
    }

    fn yield_point(&mut self) {
        let mut inner = self.heap.borrow_mut();
        if !inner.requested_gc {
            return;
        }

        debug!("Performing GC");
        inner.requested_gc = false;
        inner.global_mark_state = !inner.global_mark_state;
        let desired_mark_state = inner.global_mark_state;

        let mut tracer = MarkSweepTracer::new(self.clone(), desired_mark_state);
        trace!("Tracing shared roots");
        self.roots.borrow().trace(&mut tracer);
        trace!("Found a total of {} objects", tracer.traced);

        let bytes_cleared = inner.perform_sweep();
        debug!(
            "Performed cleanup which cleared {} bytes of space",
            bytes_cleared
        );
    }

    fn request_gc(&mut self, _collect: CollectionType) {
        trace!("Received request for GC: {:?}", _collect);

        self.heap.borrow_mut().requested_gc = true;
    }
}

impl Trace<Self> for MarkSweepGC {
    fn trace(&self, tracer: &mut MarkSweepTracer) {
        self.roots.borrow().trace(tracer)
    }
}

impl RootSource<Self> for MarkSweepGC {
    type Index = RootIndex;

    fn add_root<T>(&mut self, root: &Gc<T, Self>) -> Self::Index
    where
        T: Trace<Self>,
        Self: Alloc<T>,
    {
        self.roots.borrow_mut().add_root(root)
    }

    fn remove_by_index(&mut self, index: Self::Index) -> bool {
        self.roots.borrow_mut().remove_by_index(index)
    }

    fn remove_root<T>(&mut self, root: &Gc<T, Self>) -> bool
    where
        T: Trace<Self>,
        Self: Alloc<T>,
    {
        self.roots.borrow_mut().remove_root(root)
    }
}

#[cfg(test)]
mod test {
    use crate::MarkSweepGC;
    use gc_api::alloc::{Allocator, CollectionType};
    use gc_benchmark_utils::tree::Node;
    use log::info;
    use log::LevelFilter;
    use std::sync::Once;

    fn setup_logging() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            pretty_env_logger::formatted_builder()
                .format_timestamp(None)
                .filter_level(LevelFilter::Trace)
                .init();
        });
    }

    #[test]
    pub fn build_tree() {
        setup_logging();
        // 1MB heap
        let mut heap = MarkSweepGC::with_capacity(1 << 20);

        for height in 0..14 {
            let node = Node::build_tree_bottom_up(&mut heap, height);
            info!("Height {}: {:?}", height, &heap);

            if let Some(node_value) = node {
                let guard = node_value.get(&heap);
                assert!(guard.verify_tree(&heap));
                info!("   Passed Heap Verification");
            }
        }

        info!("{:?}", &heap);
    }

    #[test]
    pub fn gc_tree() {
        setup_logging();
        // 1MB heap
        let mut heap = MarkSweepGC::with_capacity(1 << 20);
        let node = Node::build_tree_bottom_up(&mut heap, 14);
        heap.request_gc(CollectionType::Full);
        heap.yield_point();

        let inner = heap.heap.borrow();
        assert_eq!(inner.start, inner.cursor);
    }

    #[test]
    pub fn many_small() {
        setup_logging();
        let mut heap = MarkSweepGC::with_capacity(1 << 20);

        let mut data = 1;
        for round in 0..20 {
            let mut verify_data = data;
            let node = Node::create_tree_impl(&mut heap, &mut data, 12);
            info!("Round {}: {:?}", round, &heap);

            if let Some(node_value) = node {
                let guard = node_value.get(&heap);
                assert!(guard.verify_tree_impl(&heap, &mut verify_data));
                info!("   Passed Heap Verification");
            }
        }

        info!("{:?}", &heap);
    }
}
