svg-util, a small SVG helper library written in Rust
====================================================

This crate provides some helper data structures and functions for working with SVG files.

Features
--------
 - Parsing and printing of path strings, optionally into/from geometric primitives.
 - Parsing `<line>`, `<rect>` (without rounded corners), `<circle>` and `<ellipse>` into geometric primitives.

License
-------
The code is licensed under [Creative Commons Zero v1.0 Universal](http://creativecommons.org/publicdomain/zero/1.0/legalcode), which essentially means that it's in public domain.

Version history
---------------

Planned features
----------------

 - Implement `FromStr` for `Primitive`.
 - Support to parse `<polygon>`s and `<polyline>`s into `Primitive`s.
 - Support to convert `<rect>`s with rounded corners into `Primitive`s.
 - Support for parsing and applying `transform`s.
 - Extensive docmentation.
 - Extensive testing (Maybe also using afl-rs).
 - Use `[feature]`s to make compilation time and binary size better for users. Optimized output, precision could be made into features that can be turned off.
 - Benchmarking and optimisation.
 - Maybe a C API.
