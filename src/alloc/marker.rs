//! Marker traits which effect the functionality allowed on a GcHandle.

/// Can a type be safely transmuted without the help of the allocator?
pub trait BlindTransmute {}

/// Can a handle be upgraded from a [Gc] to a [GcMut]?
pub trait UpgradeHandle {}
