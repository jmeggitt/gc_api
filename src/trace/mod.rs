use crate::{Alloc, Gc};

mod trace_impls;
pub mod roots;

pub trait TracingAllocator {
    type Tracer<'a>: 'a + Tracer<'a, Self>;
}

/// A simple and versatile Trace trait modeled after `std::hash::Hash`.
pub trait Trace<A: TracingAllocator> {
    fn trace(&self, tracer: &mut A::Tracer<'_>);

    /// Honestly, I'm a little unsure of if this is necessary or not. However, `std::hash::Hash` has
    /// it so I will trust in their design logic.
    fn trace_slice(data: &[Self], tracer: &mut A::Tracer<'_>)
    where
        Self: Sized,
    {
        for item in data {
            item.trace(tracer);
        }
    }
}

/// Not sure what I want this to be, but I thought I might as well leave it as a stub to make
/// its usage more explicit.
pub trait Tracer<'a, A: ?Sized>: Sized
where
    A: TracingAllocator<Tracer<'a> = Self>,
{
    fn trace_obj<T: ?Sized + Trace<A>>(&mut self, obj: &Gc<T, A>)
    where
        A: Alloc<T>;
}

