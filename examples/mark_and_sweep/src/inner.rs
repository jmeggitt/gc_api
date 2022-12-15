use crate::ptr_arena::PtrArena;
use crate::roots::MarkSweepTracer;
use crate::MarkSweepGC;
use gc_api::alloc::CollectionType;
use gc_api::error::{Error, ErrorKind};
use gc_api::mark::Mark;
use gc_api::trace::Trace;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::mem::size_of;
use std::ptr;
use log::trace;

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
        let layout = Layout::from_size_align(len, 4096).unwrap();
        trace!("Allocating heap: {:?}", layout);
        let start = unsafe { System.alloc(layout) };
        trace!("Allocated heap to: {:p}", start);

        // println!("Start: {:p}, Layout: {:?}", start, layout);

        MarkSweepImpl {
            start,
            end: (start as usize + len) as *mut u8,
            cursor: start,
            ref_table: PtrArena::new(),
            global_mark_state: false,
            requested_gc: false,
        }
    }

    pub fn contains_ptr(&self, ptr: *mut u8) -> bool {
        ptr >= self.start && ptr <= self.end
    }

    pub fn capacity(&self) -> usize {
        self.end as usize - self.start as usize
    }

    pub fn usage(&self) -> usize {
        self.cursor as usize - self.start as usize
    }

    pub fn free_space(&self) -> usize {
        self.end as usize - self.cursor as usize
    }

    pub fn alloc(&mut self, layout: Layout) -> Result<*mut u8, Error> {
        // let mark_word_space = FIXED_ALIGN.max(size_of::<MarkWord>());
        //
        // let layout = layout.align_to(FIXED_ALIGN).unwrap().pad_to_align();
        if layout.align() > FIXED_ALIGN {
            return Err(Error::from(ErrorKind::UnsupportedAlignment));
        }

        // let len = layout.size().min(FIXED_ALIGN) + mark_word_space;
        // if (self.end as usize - self.cursor as usize) < len {
        //     return Err(Error::from(ErrorKind::OutOfMemory));
        // }

        let (mark_ptr, new_obj) = next_obj(self.cursor);
        let new_cursor = (new_obj as usize + layout.size()) as *mut u8;

        // assert!(mark_ptr as *mut u8 >= self.start, "Mark pointer {:p} < Heap start {:p}", mark_ptr, self.start);
        // assert!(new_obj > self.start);

        if new_cursor > self.end {
            return Err(Error::from(ErrorKind::OutOfMemory));
        }

        // let new_ptr = (self.cursor as usize + mark_word_space) as *mut u8;
        // self.cursor = (self.cursor as usize + layout.size()) as *mut u8;
        self.cursor = new_cursor;

        // let mark_ptr = (new_ptr as usize - size_of::<MarkWord>()) as *mut MarkWord;
        unsafe {
            // Write object mark
            let mark = MarkWord::for_len(layout.size());
            mark.store_mark_state(self.global_mark_state);
            *mark_ptr = mark;

            let ref_table_slot = self.ref_table.claim_slot();
            *ref_table_slot = new_obj;


            // trace!("Allocated object ({:?}): {:p} -> {:p}", layout, ref_table_slot, new_obj);
            Ok(ref_table_slot as *mut u8)
        }
    }

    pub fn perform_sweep(&mut self) -> usize {
        trace!("Sweeping heap [start: {:p}, cursor: {:p}, end: {:p}]", self.start, self.cursor, self.end);
        let mut compressed = self.start;
        let mut cursor = self.start;

        while cursor < self.cursor {
            unsafe {
                let (mark_word, obj_ptr) = next_obj(cursor);
                let len = (*mark_word).object_len();

                let (dst_mark, dst_obj) = next_obj(compressed);

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
        }

        assert_eq!(cursor, self.cursor);
        self.cursor = compressed;

        cursor as usize - compressed as usize
    }
}

const FIXED_ALIGN: usize = 8;

#[inline(always)]
fn next_obj(pos: *mut u8) -> (*mut MarkWord, *mut u8) {
    let offset = ((pos as usize + size_of::<MarkWord>()) as *mut u8).align_offset(FIXED_ALIGN);

    let obj = (pos as usize + offset + size_of::<MarkWord>()) as *mut u8;
    let mark_word = (obj as usize - size_of::<MarkWord>()) as *mut MarkWord;

    (mark_word, obj)
}

// fn next_slot(position: *mut u8, len: usize) -> *mut u8 {
//
//     position.align_offset(FIXED_ALIGN)
//
//
// }

impl Drop for MarkSweepImpl {
    fn drop(&mut self) {
        let len = self.end as usize - self.start as usize;
        let layout = Layout::from_size_align(len, 4096).unwrap();

        // println!("Attempting drop:");
        // println!("Start: {:p}, Layout: {:?}", self.start, layout);
        trace!("Dropping heap [Start: {:p}, Layout: {:?}]", self.start, layout);

        unsafe {
            System.dealloc(self.start, layout);
        }
        // println!("Performed drop!");
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct MarkWord {
    mark: Cell<usize>,
}

impl MarkWord {
    const MARK_BIT: usize = 1 << (usize::BITS - 1);

    fn for_len(len: usize) -> Self {
        MarkWord {
            mark: Cell::new(len),
        }
    }

    fn object_len(&self) -> usize {
        self.mark.get() & !Self::MARK_BIT
    }
}

impl Mark for MarkWord {
    fn load_mark_state(&self) -> bool {
        self.mark.get() & Self::MARK_BIT != 0
    }

    fn store_mark_state(&self, state: bool) {
        if state {
            self.mark.set(self.object_len() | Self::MARK_BIT);
        } else {
            self.mark.set(self.object_len());
        }
    }
}
