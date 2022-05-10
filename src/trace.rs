/// A simple and versatile Trace trait modeled after `std::hash::Hash`.
pub trait Trace {
    fn trace<T: Tracer>(&self, tracer: &mut T);

    /// Honestly, I'm a little unsure of if this is necessary or not. However, `std::hash::Hash` has
    /// it so I will trust in their design logic.
    fn trace_slice<T: Tracer>(data: &[Self], tracer: &mut T)
    where
        Self: Sized,
    {
        for item in data {
            item.trace(tracer);
        }
    }
}

// TODO: Implement Trace for everything under the sun (slices, tuples, standard library collections, etc.)

/// Not sure what I want this to be, but I thought I might as well leave it as a stub to make
/// its usage more explicit.
pub trait Tracer {}
