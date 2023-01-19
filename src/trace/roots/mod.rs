use crate::Alloc;
use crate::Gc;

pub mod collections;
pub use collections::*;

/// A source for gc roots which can be iterated over. Root sources are assumed to be unordered and
/// may contain duplicate values.
pub trait RootStorage<A> {
    type Index;

    fn remove_by_index(&mut self, index: Self::Index) -> bool;
}

pub trait GcRootStorage<T, A: Alloc<T>>: RootStorage<A> {
    fn add_root(&mut self, root: &Gc<T, A>) -> Self::Index;
    fn remove_root(&mut self, root: &Gc<T, A>) -> bool;
}
