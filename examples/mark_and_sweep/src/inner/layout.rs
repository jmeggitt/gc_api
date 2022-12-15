use crate::inner::mark::MarkWord;
use std::mem::size_of;
use std::ptr::NonNull;

/// To simplify the process of dealing with alignment, we align everything to a word of memory. This
/// is the same approach that malloc uses for alignment.
pub const FIXED_ALIGN: usize = 8;

/// Type alias for readability. Really it is just a placeholder value for some bytes.
pub type Object = u8;

pub type ObjectHandle = NonNull<NonNull<Object>>;

#[inline(always)]
pub fn next_obj(pos: *mut Object) -> (*mut MarkWord, *mut Object) {
    let offset = ((pos as usize + size_of::<MarkWord>()) as *mut u8).align_offset(FIXED_ALIGN);

    let obj = (pos as usize + offset + size_of::<MarkWord>()) as *mut u8;
    let mark_word = (obj as usize - size_of::<MarkWord>()) as *mut MarkWord;

    (mark_word, obj)
}

#[inline(always)]
pub unsafe fn mark_for_obj<'a>(obj: *mut Object) -> &'a MarkWord {
    &*(obj.offset(-(size_of::<MarkWord>() as isize)) as *const MarkWord)
}
