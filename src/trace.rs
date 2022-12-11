use crate::{Alloc, AllocMut, Gc, GcMut};

/// A simple and versatile Trace trait modeled after `std::hash::Hash`.
pub trait Trace<A: ?Sized> {
    fn trace<T: Tracer<A>>(&self, tracer: &mut T);

    /// Honestly, I'm a little unsure of if this is necessary or not. However, `std::hash::Hash` has
    /// it so I will trust in their design logic.
    fn trace_slice<T: Tracer<A>>(data: &[Self], tracer: &mut T)
    where
        Self: Sized,
    {
        for item in data {
            item.trace(tracer);
        }
    }
}

/// This implementation simply switches the underying method from the tracer to consume the item.
impl<T, A: Alloc<T>> Trace<A> for Gc<T, A> {
    fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
        tracer.trace_obj(self)
    }
}

/// This implementation simply switches the underying method from the tracer to consume the item.
impl<T, A: AllocMut<T>> Trace<A> for GcMut<T, A> {
    fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
        tracer.trace_mut_obj(self)
    }
}

/// Not sure what I want this to be, but I thought I might as well leave it as a stub to make
/// its usage more explicit.
pub trait Tracer<A: ?Sized> {
    fn trace_obj<T: ?Sized>(&mut self, obj: &Gc<T, A>)
    where
        A: Alloc<T>;

    fn trace_mut_obj<T: ?Sized>(&mut self, obj: &GcMut<T, A>)
    where
        A: AllocMut<T>;
}

mod impls {
    use crate::trace::{Trace, Tracer};
    use std::marker::PhantomData;
    use std::mem::ManuallyDrop;
    use std::num::*;
    use std::ptr::NonNull;
    use std::rc::Rc;
    use std::sync::atomic::*;
    use std::sync::Arc;

    macro_rules! impl_trace_nop {
        ($($(#[$($macros:tt)+])* $name:ty)+) => {
            $(
                $(#[$($macros)+])*
                impl<A> Trace<A> for $name {
                    #[inline(always)]
                    fn trace<T: Tracer<A>>(&self, _: &mut T) {}
                }
            )+
        };
    }

    // Implement empty trace for primitive and integer types
    impl_trace_nop! { () i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool char }
    impl_trace_nop! { NonZeroI8 NonZeroI16 NonZeroI32 NonZeroI64 NonZeroI128 NonZeroIsize
    NonZeroU8 NonZeroU16 NonZeroU32 NonZeroU64 NonZeroU128 NonZeroUsize }
    impl_trace_nop! {
        #[cfg(target_has_atomic = "8")] AtomicU8
        #[cfg(target_has_atomic = "16")] AtomicU16
        #[cfg(target_has_atomic = "32")] AtomicU32
        #[cfg(target_has_atomic = "64")] AtomicU64
        #[cfg(target_has_atomic = "128")] AtomicU128
        #[cfg(target_has_atomic = "ptr")] AtomicUsize

        #[cfg(target_has_atomic = "8")] AtomicI8
        #[cfg(target_has_atomic = "16")] AtomicI16
        #[cfg(target_has_atomic = "32")] AtomicI32
        #[cfg(target_has_atomic = "64")] AtomicI64
        #[cfg(target_has_atomic = "128")] AtomicI128
        #[cfg(target_has_atomic = "ptr")] AtomicIsize

        #[cfg(target_has_atomic = "8")] AtomicBool
    }

    #[cfg(target_has_atomic = "ptr")]
    impl<A, P> Trace<A> for AtomicPtr<P> {
        #[inline(always)]
        fn trace<T: Tracer<A>>(&self, _: &mut T) {}
    }

    impl_trace_nop! { String str }

    impl<A, P: ?Sized> Trace<A> for PhantomData<P> {
        #[inline(always)]
        fn trace<T: Tracer<A>>(&self, _: &mut T) {}
    }

    impl<A, P: ?Sized> Trace<A> for *const P {
        #[inline(always)]
        fn trace<T: Tracer<A>>(&self, _: &mut T) {}
    }

    impl<A, P: ?Sized> Trace<A> for *mut P {
        #[inline(always)]
        fn trace<T: Tracer<A>>(&self, _: &mut T) {}
    }

    impl<A, P: ?Sized> Trace<A> for NonNull<P> {
        #[inline(always)]
        fn trace<T: Tracer<A>>(&self, _: &mut T) {}
    }

    /// This is a wierd one. Is tracing or not-tracing more in the spirit of manually drop? At the
    /// moment I am leaving it as a black box that does not propogate anything related to dropping
    /// or freeing a resource.
    impl<A, T: Trace<A>> Trace<A> for ManuallyDrop<T> {
        #[inline(always)]
        fn trace<B: Tracer<A>>(&self, _: &mut B) {}
    }

    macro_rules! impl_trace_tuple {
        ($($name:ident)+) => {
            impl<Alloc, $($name: Trace<Alloc>),+> Trace<Alloc> for ($($name,)+)
                where last_type!($($name,)+): ?Sized,
            {
                #[inline]
                #[allow(non_snake_case)]
                fn trace<Trace: Tracer<Alloc>>(&self, tracer: &mut Trace) {
                    let ($(ref $name,)+) = *self;
                    $($name.trace(tracer);)+
                }
            }
        };
    }

    macro_rules! last_type {
        ($last:ident,) => {$last};
        ($_:ident, $($last:ident,)+) => { last_type!($($last,)+) }
    }

    impl_trace_tuple! { A }
    impl_trace_tuple! { A B }
    impl_trace_tuple! { A B C }
    impl_trace_tuple! { A B C D }
    impl_trace_tuple! { A B C D E }
    impl_trace_tuple! { A B C D E F }
    impl_trace_tuple! { A B C D E F G }
    impl_trace_tuple! { A B C D E F G H }
    impl_trace_tuple! { A B C D E F G H I }
    impl_trace_tuple! { A B C D E F G H I J }
    impl_trace_tuple! { A B C D E F G H I J K }
    impl_trace_tuple! { A B C D E F G H I J K L }

    impl<A, T: ?Sized + Trace<A>> Trace<A> for &T {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            (**self).trace(tracer)
        }
    }

    impl<A, T: ?Sized + Trace<A>> Trace<A> for &mut T {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            (**self).trace(tracer)
        }
    }

    impl<A, T: Trace<A>> Trace<A> for [T] {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace_slice(self, tracer)
        }
    }

    impl<A, T: Trace<A>, const N: usize> Trace<A> for [T; N] {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace_slice(self, tracer)
        }
    }

    impl<A, T: ?Sized + ToOwned + Trace<A>> Trace<A> for std::borrow::Cow<'_, T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace(&**self, tracer)
        }
    }

    // Implement trace for std collections and types
    impl<A, T: Trace<A>> Trace<A> for Option<T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            if let Some(x) = self {
                x.trace(tracer)
            }
        }
    }

    impl<A, T: Trace<A>> Trace<A> for Box<T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace(&**self, tracer)
        }
    }

    impl<A, T: Trace<A>> Trace<A> for Rc<T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace(&**self, tracer)
        }
    }

    impl<A, T: Trace<A>> Trace<A> for Arc<T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace(&**self, tracer)
        }
    }

    impl<A, T: Trace<A>> Trace<A> for Vec<T> {
        #[inline]
        fn trace<B: Tracer<A>>(&self, tracer: &mut B) {
            Trace::trace_slice(&self[..], tracer)
        }
    }

    // TODO: Should I implement Trace for all `std::collections`? I am tempted to implement it over
    // `IntoIter`, but that may lead to issues.
}
