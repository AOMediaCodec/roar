# Change Log

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Changes in the repo not yet in a release will appear here.

### [Added]

-   Split the main library entrypoint into `aom_roar` into `aom_roar` (for C++)
    and `aom_roar_rlib` (for Rust) to allow Bazel integration with either Rust
    or C++ and to allow the Rust integration tests to run with Bazel.

## [1.0.0] - 2026-10-15

### [Added]

-   [Matching the new C-API functions in OAR](https://github.com/AOMediaCodec/oar/pull/40),
    there are new API methods for controlling the limiter:
    `roar_set_limiter_threshold`, `roar_get_limiter_delay`, and `roar_flush`.

### [Changed]

-   Input audio is now discarded after a call to `roar_render`.
-   Matching [a change in OAR](https://github.com/AOMediaCodec/oar/pull/35),
    animated (linear, bezier) gains are now interpolated in the decibel domain.
-   Matching
    [the new limiter in OAR](https://github.com/AOMediaCodec/oar/pull/40), the
    simple limiter is replaced with a look-ahead limiter.
-   Using `roar_set_metadata_unit_to_process` was accepted for binaural output
    but was not actually applied. It is now explicitly allowed (in the README),
    works correctly, and is tested.
-   When output format is **binaural** and audio element **object positions**
    are updated with `roar_update_audio_element_metadata` (C API) or
    `RoarRenderer::update_element_positions` (Rust API) before the first render,
    the object is now instantly updated to the new position rather than ramping
    to the position over the first render. This behavior was changed in OAR
    ([oar/issues/21](https://github.com/AOMediaCodec/oar/issues/21),
    [oar/pull/24](https://github.com/AOMediaCodec/oar/pull/24)).

### [Fixed]

-   [Matching the fixes in OAR](https://github.com/AOMediaCodec/oar/pull/14),
    when binaural output is requested and the output sample rate is not 48kHz,
    the output audio is now appropriately scaled and the interpolation filter
    tail is no longer truncated.

## [0.1.0] - 2026-09-23

### [Added]

-   Initial release, supporting a C API compatible with OAR's C API.
