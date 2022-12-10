//! # Brainstorming notes for Access
//!
//! Access traits need to fit a couple key requirements that make it problematic.
//!
//! ## Considerations:
//!  - Access must be able to return an error if the GC is able to detect any issues.
//!  - A guard may be necessary if a GC needs to track when an access is in progress. Guard types
//!    can be generic, but it might not be necessary. It may be easier to implement a GC if there
//!    was an access end function that was called with a provided accessor. Are there any situations
//!    where this would this limit the ability of implementors?
//!  - A handle should be treated similarly to an Arc (or Rc), so access can only be immutable under
//!    normal circumstances.
//!  - Some mark words come with mutex functionality (A notable one being Oracle's G1). To make
//!    this functionality usable, either the mark word needs to be accessible or the access trait
//!    needs to provide this functionality.
//!  - How can we differentiate when an object needs to be manually wrapped in a mutex vs can be
//!    used as-is.
//!
//! ## Working Ideas/considerations
//!  - There needs to be a way to easily initialize types with or without interior mutability.
//!

use crate::error::Error;
use crate::Alloc;
use std::ops::{Deref, DerefMut};

/// TODO: Is there any reason why this should not be merged with Alloc<T>?
pub trait Access<T: ?Sized>: Alloc<T> {
    type Guard: Deref<Target = T>;

    unsafe fn access(&self, handle: &Self::RawHandle) -> Result<Self::Guard, Error>;
}

pub trait AccessMut<T: ?Sized>: Alloc<T> {
    type Guard: DerefMut<Target = T>;

    unsafe fn access_mut(&self, handle: &Self::RawHandle) -> Result<Self::Guard, Error>;
}

/// A Simple wrapper for an immutable reference which satisfies the requirements for guards.
pub struct ThinGuard<'a, T>(&'a T);

impl<'a, T> From<&'a T> for ThinGuard<'a, T> {
    fn from(x: &'a T) -> Self {
        ThinGuard(x)
    }
}

impl<'a, T> Deref for ThinGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
