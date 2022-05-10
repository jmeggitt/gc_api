# `gc_api`
A trait based `Rust` API for a generic garbage collector. This project currently exists to formulate what such
an API might look like while supporting a wide range of garbage collector types/implementations.

## The Safety Issue
How can some pointer `Gc<T>` to an element within the heap be used safely? The problem has two parts:
- Ensure `Gc<T>` either does not outlive or is not used beyond heap lifetime
- ABA problem (How can we know the object we want has not been garbage collected, and we are now pointing to a
  different object at the same location)
    - We must either defer reclamation of an object until no references to it remain or have a means of differentiating
      objects when the contents of the heap changes. Deferring reclamation is not really an option since it would
      require keeping track of how many are taken remain and that would prevent pointer from implementing `Copy`.
    
### Defragmentation
To make defragmentation possible, an indirect pointer is required. Objects hold pointers into an unmoving reference
table with pointers to active objects. Upon defragmentation the reference table is updated to reflect the new locations
of objects. The only alternatives are to either find and update all referencing pointers or use an indexing system which
dynamically resolves the object's location upon access.

### ABA
To solve the ABA problem, we only have a limited number of options at our disposal. Deferred reclamation is not possible
since we have no way to determine if deferral is necessary while keeping the pointer `Copy`. The only option left to us
is to provide some way of distinguishing objects. This leaves us with the option to use tagged states. Practically
speaking, tags would likely take the form of a generation counter. Both the pointers and objects would need to store
this tag. However, placement of the tag can be tricky since depending on the allocation method, pointers to reclaimed
objects may be aligned to the start of whatever now uses that part of the heap. If we want to use direct object pointers
we need a way to determine the offset of the tag. This means we would need to use multiple fixed-object-size heaps so
the tag is always a fixed offset from an object pointer no matter the gc state. The other approach is to use indirect
pointers instead. The reference table could be expanded to hold object tags with data pointers. This does not limit how
data is placed into the heap or increase the size of heap objects.




