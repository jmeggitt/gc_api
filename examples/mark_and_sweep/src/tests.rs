use crate::MarkSweepGC;
use gc_api::alloc::{Allocator, CollectionType};
use gc_benchmark_utils::tree::Node;

// Use a heap of 1MB for tests due to simplicity.
const HEAP_SIZE: usize = 1 << 20;

#[test]
pub fn build_tree() {
    let mut heap = MarkSweepGC::with_capacity(HEAP_SIZE);

    for height in 0..14 {
        let node = Node::build_tree_bottom_up(&mut heap, height);

        if let Some(node_value) = node {
            let guard = node_value.get(&heap);
            assert!(guard.verify_tree(&heap));
        }
    }
}

#[test]
pub fn gc_tree() {
    let mut heap = MarkSweepGC::with_capacity(HEAP_SIZE);

    // Create a simple node, but do not root it
    let node = Node::build_tree_bottom_up(&mut heap, 14);

    // Perform a full garbage collection
    heap.request_gc(CollectionType::Full);
    heap.yield_point();

    // Verify we no longer have any data on the heap
    // let inner = heap.heap.borrow();
    // assert_eq!(heap.alloc.0.start, heap.alloc.0.cursor);
}

#[test]
pub fn many_small() {
    let mut heap = MarkSweepGC::with_capacity(HEAP_SIZE);

    let mut data = 1;
    for _ in 0..20 {
        let mut verify_data = data;
        let node = Node::create_tree_impl(&mut heap, &mut data, 12);

        if let Some(node_value) = node {
            let guard = node_value.get(&heap);
            assert!(guard.verify_tree_impl(&heap, &mut verify_data));
        }
    }
}
