# Examples

This folder contains examples of how `gc_api` can be implemented for a simple GC as well as used in projects. In the
future, some [canonical GCs] may be implemented here to provide benchmark comparisons for other GC implementations.

> **Note:** At the moment, `gc_api` is unfinished and these example projects serve more as a practical test of how well
> `gc_api` is able to serve the needs of a GC. While they may help as examples, they should be considered work in
> progress tests at the moment.

## `mark_and_sweep`
An extremely simple bare-bones mark and sweep garbage collector. At the moment, the current implementation is a bit
sloppy as it is only being used for testing purposes.


<!-- This link was simply the first one I found which describes a couple canonical GC implementations. I have not read
through the entirety of this page. -->
[canonical GCs]: https://1library.net/article/canonical-garbage-collectors-performance-reference-counting-conservative-garbage.dzxre7dz