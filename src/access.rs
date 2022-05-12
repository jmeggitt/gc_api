use crate::Alloc;


pub trait Access<'a, 'b, T: ?Sized>: Alloc<T> {
    type Target: 'b;

    fn access(handle: &'a Self::RawHandle) -> Self::Target;
}

pub trait AccessWithAllocator<'a, 'b, T: ?Sized>: Alloc<T> {
    type Target: 'b;

    fn access_with(&'a self, handle: &'a Self::RawHandle) -> Self::Target;
}

impl<'a, 'b, T: ?Sized, A: Access<'a, 'b, T>> AccessWithAllocator<'a, 'b, T> for A {
    type Target = <Self as Access<'a, 'b, T>>::Target;

    fn access_with(&'a self, handle: &'a Self::RawHandle) -> Self::Target {
        <Self as Access<'a, 'b, T>>::access(handle)
    }
}
