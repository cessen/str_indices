# Changelog


## [Unreleased]


## [0.4.0] - 2022-05-25

### New Features
- Added "simd" feature flag to allow disabling simd usage in the library.


## [0.3.2] - 2022-03-22

### Performance
- Substantially improved performance for `chars::count()` and `lines_lf::count_breaks()` on very short strings, in some cases up to 2x faster.


## [0.3.1] - 2022-03-14

### Performance
- `utf16::to_byte_idx()` is actually optimized now (it was the last remaining non-optimized function), for a ~6x improvement in speed.
- Substantially improved performance on Apple M1 platforms (over 6x for some functions).
- Mild-to-moderate performance improvements across the board on x86/64.


## [0.3.0] - 2022-03-12

### New Features
- Added `lines_lf` module, a line-feed-only variant of the `lines` module.
- Added `lines_crlf` module, a line feed and carriage return variant of the `lines` module.

### Test Suite
- Added property testing.
- Added fuzzing.


## [0.2.0] - 2022-03-11

- Major clean up of the code and API.
- Added minimal documentation.


## [0.1.0] - 2022-03-11

- First release.
- Split off from [Ropey](https://crates.io/crates/ropey).


[Unreleased]: https://github.com/cessen/str_indices/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/cessen/str_indices/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/cessen/str_indices/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/cessen/str_indices/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/cessen/str_indices/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/cessen/str_indices/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/cessen/str_indices/releases/tag/v0.1.0
