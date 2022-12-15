//! Honestly, it is a somewhat sloppy implementation, but I chose to just do a more c-like approach.

use std::ops::Index;
use std::ptr::null_mut;

/// A super simple arena which is used to act as a reference table. It functions similarly to the
/// generational_arena crate but with the added pros/cons:
///  - All items are pinned in memory allowing self-referential structures
///  - Pointers are used instead of indices for easier indexing
///  - Higher memory efficiency. While the generational_arena crate uses enums (tagged-unions) this
///    implementation uses plain pointers (comparable to a regular union).
///  - Far less safety since it relies upon raw pointers to function
///
/// Each slot has 2 possible states:
///  - Occupied. The pointer is being used to point to some object. We assume that this pointer is
///    unique, non-null, and points do a location outside of this data structure.
///  - Free. In this case, the pointer points to the next free position. If there are no more free
///    positions it remains null indicating that a new chunk must be allocated.
pub struct PtrArena {
    free_ptr: *mut *mut u8,
    chunks: Vec<PtrArenaChunk>,
}

impl PtrArena {
    pub fn new() -> Self {
        let first_slab = PtrArenaChunk::new_linked_block();

        PtrArena {
            free_ptr: first_slab.start_ptr(),
            chunks: vec![first_slab],
        }
    }

    pub fn contains_ptr(&self, ptr: *mut u8) -> bool {
        self.chunks.iter().any(|x| x.contains_ptr(ptr))
    }

    pub unsafe fn claim_slot(&mut self) -> *mut *mut u8 {
        let slot = self.free_ptr;
        let next_slot = *slot as *mut *mut u8;

        if next_slot.is_null() {
            let new_slab = PtrArenaChunk::new_linked_block();
            self.free_ptr = new_slab.start_ptr();
            self.chunks.push(new_slab);
        } else {
            self.free_ptr = next_slot;
        }

        assert!(self.contains_ptr(slot as *mut u8));
        slot
    }

    pub unsafe fn free_slot_by_value(&mut self, value: *mut u8) {
        for chunk in &mut self.chunks {
            for x in &mut *chunk.ptr {
                if *x == value {
                    *x = self.free_ptr as *mut u8;
                    self.free_ptr = x as *mut *mut u8;
                    return
                }
            }
        }

        panic!("Failed to find slot to free")
    }

    pub unsafe fn update_slot_by_value(&mut self, previous: *mut u8, new: *mut u8) {
        for chunk in &mut self.chunks {
            for x in &mut *chunk.ptr {
                if *x == previous {
                    *x = new;
                    return
                    // *x = self.free_ptr as *mut u8;
                    // self.free_ptr = x as *mut *mut u8;
                }
            }
        }
        panic!("Failed to find slot to update")
    }

    pub unsafe fn free_slot(&mut self, slot: *mut *mut u8) {
        *slot = self.free_ptr as *mut u8;
        self.free_ptr = slot;
    }
}

#[repr(transparent)]
struct PtrArenaChunk {
    ptr: Box<[*mut u8; 1024]>
}

impl PtrArenaChunk {

    pub fn contains_ptr(&self, ptr: *mut u8) -> bool {
        ptr >= &self.ptr[0] as *const _ as *mut u8 && ptr <= &self.ptr[1023] as *const _ as *mut u8
    }

    fn start_ptr(&self) -> *mut *mut u8 {
        &self.ptr[0] as *const _ as *mut *mut u8
    }

    fn new_linked_block() -> Self {
        // Allocate chunk
        let mut boxed_ptrs: Box<[*mut u8; 1024]> = vec![null_mut(); 1024].into_boxed_slice().try_into().unwrap();

        // Fill in pointers 0-1023 to point to next cell.
        for index in 0..1023 {
            boxed_ptrs[index] = &boxed_ptrs[index + 1] as *const _ as *mut u8;
        }

        PtrArenaChunk {
            ptr: boxed_ptrs
        }
    }
}
