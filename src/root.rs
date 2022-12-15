use crate::trace::{Trace, TracingAllocator};
use crate::Alloc;
use crate::Gc;

/// A source for gc roots which can be iterated over. Root sources are assumed to be unordered and
/// may contain duplicate values.
pub trait RootSource<A: TracingAllocator>: Trace<A> {
    type Index;

    fn add_root<T>(&mut self, root: &Gc<T, A>) -> Self::Index
    where
        T: Trace<A>,
        A: Alloc<T>;

    fn remove_root<T>(&mut self, root: &Gc<T, A>) -> bool
    where
        T: Trace<A>,
        A: Alloc<T>;

    fn remove_by_index(&mut self, index: Self::Index) -> bool;
}
