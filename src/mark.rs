use lock_api::RawMutex;

/// A mark type which is used to determine if a type is in use.
///
/// > **Notes:** There seem to be two primary types of object mark depending on implementation.
/// >  - Marks which are unset on an initial pass before being set during the tracing pass.
/// >  - Marks which flip the state of the mark between traces. This requires new objects be
///      initialized to the current mark state, but does not require an un-marking pass on tracing.
pub trait Mark {
    /// Read the current state of the mark.
    fn load_mark_state(&self) -> bool;

    /// Set the mark to a new state.
    fn store_mark_state(&self, state: bool);

    /// Set the mark to a new state. Upon being set, the previous mark state should be returned.
    fn swap_mark_state(&self, state: bool) -> bool {
        let previous = self.load_mark_state();
        self.store_mark_state(state);
        previous
    }
}

/// In many cases, a GC handle `Gc<T>` will function similarly to an `Arc<T>` in how it provides
/// shared immutable access to some data. However since a mark often only requires a single bit in
/// an object's header, it can make sense to use the remaining space for a mutex. This can allow a
/// `Gc<T>` to provide interior mutability without requiring all object be wrapped in an explicit
/// mutex.
pub trait LockingMark: Mark + RawMutex {}
