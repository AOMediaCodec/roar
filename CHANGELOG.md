#Change Log

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Changes in the repo not yet in a release will appear here.

### [Changed]

- When output format is **binaural** and audio element **object positions** are
  updated with `roar_update_audio_element_metadata` (C API) or
  `RoarRenderer::update_element_positions` (Rust API) before the first render,
  the object is now instantly updated to the new position rather than ramping to
  the position over the first render. This behavior was changed in OAR
  ([oar/issues/21](https://github.com/AOMediaCodec/oar/issues/21),
  [oar/pull/24](https://github.com/AOMediaCodec/oar/pull/24)).

## [0.1.0] - 2026-09-23

### [Added]

- Initial release, supporting a C API compatible with OAR's C API.
