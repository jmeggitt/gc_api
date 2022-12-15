use gc_api::mark::Mark;
use std::cell::Cell;

#[derive(Clone)]
#[repr(transparent)]
pub struct MarkWord {
    mark: Cell<usize>,
}

impl MarkWord {
    const MARK_BIT: usize = 1 << (usize::BITS - 1);

    #[inline(always)]
    pub fn new(obj_len: usize, mark_state: bool) -> Self {
        // This should be near impossible since it would require a single object cover over half of
        // the address space.
        debug_assert_eq!(obj_len & !Self::MARK_BIT, obj_len);

        let mark_value = obj_len | ((mark_state as usize) << Self::MARK_BIT.trailing_zeros());

        MarkWord {
            mark: Cell::new(mark_value),
        }
    }

    pub fn object_len(&self) -> usize {
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
