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
use crate::{Alloc, Gc};
use std::ops::{Deref, DerefMut};

/// Marker trait to describe known accessors of data
pub trait AccessedVia<Accessor> {}

pub trait Accessor<T: ?Sized, A>: Sized
where
    A: AccessedVia<Self> + Alloc<T>,
{
    // TODO: Should Deref be replaced with Borrow?
    type Guard<'g>: Deref<Target = T>
    where
        Self: 'g;

    /// Get immutable access to
    #[inline(always)]
    fn read<'g>(&'g self, object: &'g Gc<T, A>) -> Self::Guard<'g> {
        self.try_read(object)
            .unwrap_or_else(|err| failed_access(err))
    }

    #[inline(always)]
    fn try_read<'g>(&'g self, object: &'g Gc<T, A>) -> Result<Self::Guard<'g>, Error> {
        unsafe { self.access(&object.handle) }
    }

    unsafe fn access<'g>(
        &'g self,
        handle: &'g <A as Alloc<T>>::RawHandle,
    ) -> Result<Self::Guard<'g>, Error>;
}

pub trait AccessorMut<T: ?Sized, A>: Accessor<T, A>
where
    A: AccessedVia<Self> + Alloc<T>,
{
    type GuardMut<'g>: DerefMut<Target = T>
    where
        Self: 'g;

    /// Get immutable access to
    #[inline(always)]
    fn write<'g>(&'g self, object: &'g Gc<T, A>) -> Self::GuardMut<'g> {
        self.try_write(object)
            .unwrap_or_else(|err| failed_access(err))
    }

    #[inline(always)]
    fn try_write<'g>(&'g self, object: &'g Gc<T, A>) -> Result<Self::GuardMut<'g>, Error> {
        unsafe { self.access_mut(&object.handle) }
    }

    unsafe fn access_mut<'g>(
        &'g self,
        handle: &'g <A as Alloc<T>>::RawHandle,
    ) -> Result<Self::GuardMut<'g>, Error>;
}

#[inline(never)]
#[cold]
fn failed_access(err: Error) -> ! {
    panic!("Failed to access GC object: {:?}", err)
}
