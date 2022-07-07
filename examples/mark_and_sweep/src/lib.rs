mod ptr_arena;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::mem::size_of;
use gc_api::error::{Error, ErrorKind};
use gc_api::mark::Mark;
use crate::ptr_arena::PtrArena;

pub struct MarkSweepImpl {
    start: *mut u8,
    end: *mut u8,
    cursor: *mut u8,
    ref_table: PtrArena,
}

impl MarkSweepImpl {
    pub fn with_capacity(len: usize) -> Self {
        let layout = Layout::from_size_align(len, 4096).unwrap();
        let start = unsafe { System.alloc(layout) };

        MarkSweepImpl {
            start,
            end: (start as usize + len) as *mut u8,
            cursor: start,
            ref_table: PtrArena::new(),
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Result<*mut u8, Error> {
        const FIXED_ALIGN: usize = 8;
        const MARK_WORD_SPACE: usize = FIXED_ALIGN.max(size_of::<MarkWord>());

        let layout = layout.align_to(FIXED_ALIGN).unwrap().pad_to_align();
        if layout.align() > FIXED_ALIGN {
            return Err(Error::from(ErrorKind::UnsupportedAlignment))
        }

        let len = layout.size().min(FIXED_ALIGN) + MARK_WORD_SPACE;
        if (self.end as usize - self.cursor as usize) < len {
            return Err(Error::from(ErrorKind::OutOfMemory))
        }

        let new_ptr = (self.cursor as usize + MARK_WORD_SPACE) as *mut u8;
        self.cursor = (self.cursor as usize + len) as *mut u8;

        let mark_ptr = (new_ptr as usize - size_of::<MarkWord>()) as *mut MarkWord;
        unsafe { *mark_ptr = MarkWord::for_len(len); }

        Ok(new_ptr)
    }
}

impl Drop for MarkSweepImpl {
    fn drop(&mut self) {
        let len = self.end as usize - self.start as usize;
        let layout = Layout::from_size_align(len, 4096).unwrap();

        unsafe {
            System.dealloc(self.start, layout);
        }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
struct MarkWord {
    mark: Cell<usize>,
}

impl MarkWord {
    const MARK_BIT: usize = 0x1;

    fn for_len(len: usize) -> Self {
        MarkWord { mark: Cell::new(len) }
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




