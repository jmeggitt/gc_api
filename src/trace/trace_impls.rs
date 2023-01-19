use crate::alloc::Alloc;
use crate::trace::{Trace, Tracer, TracingAllocator};
use crate::Gc;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::num::*;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::*;
use std::sync::Arc;

/// This implementation simply switches the underying method from the tracer to consume the item.
impl<T: Trace<A>, A: Alloc<T> + TracingAllocator> Trace<A> for Gc<T, A> {
    #[inline(always)]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        tracer.trace_obj(self)
    }
}

macro_rules! impl_trace_nop {
        ($($(#[$($macros:tt)+])* $name:ty)+) => {
            $(
                $(#[$($macros)+])*
                impl<A: TracingAllocator> Trace<A> for $name {
                    #[inline(always)]
                    fn trace(&self, _: &mut A::Tracer<'_>) {}
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
impl<A: TracingAllocator, P> Trace<A> for AtomicPtr<P> {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

impl_trace_nop! { String str }

impl<A: TracingAllocator, P: ?Sized> Trace<A> for PhantomData<P> {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

impl<A: TracingAllocator, P: ?Sized> Trace<A> for *const P {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

impl<A: TracingAllocator, P: ?Sized> Trace<A> for *mut P {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

impl<A: TracingAllocator, P: ?Sized> Trace<A> for NonNull<P> {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

/// This is a wierd one. Is tracing or not-tracing more in the spirit of manually drop? At the
/// moment I am leaving it as a black box that does not propogate anything related to dropping
/// or freeing a resource.
impl<A: TracingAllocator, T: Trace<A>> Trace<A> for ManuallyDrop<T> {
    #[inline(always)]
    fn trace(&self, _: &mut A::Tracer<'_>) {}
}

macro_rules! impl_trace_tuple {
        ($($name:ident)+) => {
            impl<Alloc: TracingAllocator, $($name: Trace<Alloc>),+> Trace<Alloc> for ($($name,)+)
                where last_type!($($name,)+): ?Sized,
            {
                #[inline]
                #[allow(non_snake_case)]
                fn trace(&self, tracer: &mut Alloc::Tracer<'_>) {
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

impl<A: TracingAllocator, T: ?Sized + Trace<A>> Trace<A> for &T {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        (**self).trace(tracer)
    }
}

impl<A: TracingAllocator, T: ?Sized + Trace<A>> Trace<A> for &mut T {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        (**self).trace(tracer)
    }
}

impl<A: TracingAllocator, T: Trace<A>> Trace<A> for [T] {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace_slice(self, tracer)
    }
}

impl<A: TracingAllocator, T: Trace<A>, const N: usize> Trace<A> for [T; N] {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace_slice(self, tracer)
    }
}

impl<A: TracingAllocator, T: ?Sized + ToOwned + Trace<A>> Trace<A> for std::borrow::Cow<'_, T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace(&**self, tracer)
    }
}

// Implement trace for std collections and types
impl<A: TracingAllocator, T: Trace<A>> Trace<A> for Option<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        if let Some(x) = self {
            x.trace(tracer)
        }
    }
}

impl<A: TracingAllocator, T: Trace<A>> Trace<A> for Box<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace(&**self, tracer)
    }
}

impl<A: TracingAllocator, T: Trace<A>> Trace<A> for Rc<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace(&**self, tracer)
    }
}

impl<A: TracingAllocator, T: Trace<A>> Trace<A> for Arc<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace(&**self, tracer)
    }
}

impl<A: TracingAllocator, T: Trace<A>> Trace<A> for Vec<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        Trace::trace_slice(&self[..], tracer)
    }
}

#[cfg(feature = "slab")]
impl<A: TracingAllocator, T: Trace<A>> Trace<A> for slab::Slab<T> {
    #[inline]
    fn trace(&self, tracer: &mut A::Tracer<'_>) {
        for (_, entry) in self {
            Trace::trace(entry, tracer)
        }
    }
}

// TODO: Should I implement Trace for all `std::collections`? I am tempted to implement it over
// `IntoIter`, but that may lead to issues.
