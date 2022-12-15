use crate::inner::reference_table::PtrArena;
use gc_api::error::{Error, ErrorKind};
use gc_api::mark::Mark;
use log::trace;
use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::ptr::NonNull;

use crate::inner::{layout, ObjectHandle};
use crate::inner::MarkWord;

/// Attempt to line the heap up with the page size, but we are not too worried if it is a bit off.
const HEAP_ALIGNMENT: usize = 4096;

pub struct MarkSweepImpl {
    pub start: *mut u8,
    pub end: *mut u8,
    pub cursor: *mut u8,
    pub ref_table: PtrArena,
    pub global_mark_state: bool,
    pub requested_gc: bool,
}

impl MarkSweepImpl {
    pub fn with_capacity(len: usize) -> Self {
        let layout = Layout::from_size_align(len, HEAP_ALIGNMENT).unwrap();
        trace!("Allocating heap: {:?}", layout);
        let start = unsafe { System.alloc(layout) };
        trace!("Allocated heap to: {:p}", start);

        MarkSweepImpl {
            start,
            end: (start as usize + len) as *mut u8,
            cursor: start,
            ref_table: PtrArena::new(),
            global_mark_state: false,
            requested_gc: false,
        }
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> Result<ObjectHandle, Error> {
        if layout.align() > layout::FIXED_ALIGN {
            return Err(Error::from(ErrorKind::UnsupportedAlignment));
        }

        let (mark_ptr, new_obj) = layout::next_obj(self.cursor);
        let new_cursor = new_obj.add(layout.size());

        if new_cursor > self.end {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }

        self.cursor = new_cursor;

        // Write object mark
        ptr::write(mark_ptr, MarkWord::new(layout.size(), self.global_mark_state));

        // Write object
        let ref_table_slot = self.ref_table.claim_slot();
        *ref_table_slot = new_obj;

        // TODO: Write this in a more maintainable way
        Ok(NonNull::new_unchecked(ref_table_slot as *mut _))
    }

    pub unsafe fn perform_sweep(&mut self) -> usize {
        trace!(
            "Sweeping heap [start: {:p}, cursor: {:p}, end: {:p}]",
            self.start,
            self.cursor,
            self.end
        );
        let mut compressed = self.start;
        let mut cursor = self.start;

        while cursor < self.cursor {
            let (mark_word, obj_ptr) = layout::next_obj(cursor);
            let len = (*mark_word).object_len();

            let (dst_mark, dst_obj) = layout::next_obj(compressed);

            if (*mark_word).load_mark_state() == self.global_mark_state {
                ptr::copy(obj_ptr, dst_obj, (*mark_word).object_len());
                ptr::copy(mark_word, dst_mark, 1);

                compressed = (dst_obj as usize + len) as *mut u8;
                self.ref_table.update_slot_by_value(obj_ptr, dst_obj);
            } else {
                self.ref_table.free_slot_by_value(obj_ptr);
            }

            cursor = (obj_ptr as usize + len) as *mut u8;
        }

        assert_eq!(cursor, self.cursor);
        self.cursor = compressed;

        cursor as usize - compressed as usize
    }
}

impl Drop for MarkSweepImpl {
    fn drop(&mut self) {
        let len = self.end as usize - self.start as usize;
        let layout = Layout::from_size_align(len, HEAP_ALIGNMENT).unwrap();

        trace!(
            "Dropping heap [Start: {:p}, Layout: {:?}]",
            self.start,
            layout
        );

        unsafe {
            System.dealloc(self.start, layout);
        }
    }
}
