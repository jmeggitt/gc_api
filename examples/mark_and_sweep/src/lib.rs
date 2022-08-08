use std::cell::RefCell;
use std::rc::Rc;
use gc_api::allocator::Alloc;
use gc_api::Heap;
use gc_api::error::Error;
use crate::inner::MarkSweepImpl;
use std::alloc::Layout;
use gc_api::marker::BlindTransmute;

mod ptr_arena;
mod inner;

#[derive(Clone)]
pub struct MarkSweepGC {
    heap: Rc<RefCell<MarkSweepImpl>>,
}

impl MarkSweepGC {
    /// Create a new mark and sweep GC with the given heap size.
    pub fn with_capacity(len: usize) -> Self {
        let inner = MarkSweepImpl::with_capacity(len);

        MarkSweepGC { heap: Rc::new(RefCell::new(inner)) }
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

impl<T> Alloc<T> for MarkSweepGC {
    type MutAlternative = std::sync::Mutex<T>;
    type RawHandle = *mut u8;

    type Flags = Self;

    unsafe fn try_alloc(&mut self, layout: Layout) -> Result<Self::RawHandle, Error> {
        self.heap.borrow_mut().alloc(layout)
    }
}

impl BlindTransmute for MarkSweepGC {}
