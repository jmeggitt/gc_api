use gc_api::alloc::{Accessor, Alloc, Allocator};
use gc_api::root::RootSource;
use gc_api::trace::{Trace, TracingAllocator};
use gc_api::Gc;

use crate::workload;

pub struct Node<A: Alloc<Self>> {
    left: Option<Gc<Node<A>, A>>,
    right: Option<Gc<Node<A>, A>>,
    data: u32,
}

impl<A: Alloc<Self> + TracingAllocator> Trace<A> for Node<A> {
    fn trace(&self, tracer: &mut A::Tracer) {
        self.left.trace(tracer);
        self.right.trace(tracer);
    }
}

impl<A: Alloc<Self> + TracingAllocator> Node<A> {
    pub fn build_tree_bottom_up<B>(allocator: &mut B, height: usize) -> Option<Gc<Self, A>>
    where
        B: RootSource<A> + Allocator<Alloc = A>,
    {
        let mut data = 1;
        Self::create_tree_impl(allocator, &mut data, height)
    }

    pub fn verify_tree<B>(&self, accessor: &B) -> bool
    where
        B: Accessor<Self, A>,
    {
        let mut data = 1;
        self.verify_tree_impl(accessor, &mut data)
    }

    pub fn verify_tree_impl<B>(&self, accessor: &B, data: &mut u32) -> bool
    where
        B: Accessor<Self, A>,
    {
        if let Some(left) = &self.left {
            let left_node = left.get(accessor);
            if !left_node.verify_tree_impl(accessor, data) {
                return false;
            }
        }

        if let Some(right) = &self.right {
            let right_node = right.get(accessor);
            if !right_node.verify_tree_impl(accessor, data) {
                return false;
            }
        }

        if self.data != *data {
            println!("Expected data value {:0X}, but got {:0X}", self.data, *data);
            return false;
        }

        *data = workload(*data, u32::BITS);
        true
    }

    pub fn create_tree_impl<B>(
        allocator: &mut B,
        data: &mut u32,
        height: usize,
    ) -> Option<Gc<Self, A>>
    where
        B: RootSource<A> + Allocator<Alloc = A>,
    {
        if height == 0 {
            return None;
        }

        let left = Self::create_tree_impl(allocator, data, height - 1);
        let left_root = left.as_ref().map(|x| allocator.add_root(x));
        let right = Self::create_tree_impl(allocator, data, height - 1);
        let right_root = right.as_ref().map(|x| allocator.add_root(x));

        let next = Node {
            left,
            right,
            data: *data,
        };

        *data = workload(*data, u32::BITS);

        let res = Some(allocator.alloc(next));
        left_root.map(|x| allocator.remove_by_index(x));
        right_root.map(|x| allocator.remove_by_index(x));
        res
    }
}