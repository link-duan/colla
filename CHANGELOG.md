# Changelog

All notable public changes to the Rust `colla` crate and the `colla-ot`
package are recorded here. Both artifacts always use the same version.

## [Unreleased]

### Changed

- Renamed Rust `RichInsert` to `RichContent`; no compatibility alias is kept.
- Made `RichSpan` fields private and replaced `RichText::spans()` with
  `iter_spans()` and `span_count()` so the physical storage is not public API.
- Added cached RichText scalar lengths, cumulative span lookup,
  fallible `RichText::from_spans` and `RichTextChange::try_new`; removed the
  infallible `RichText::new` constructor.
- Added indexed RichText Unicode scalar/UTF-16 coordinate conversion while
  keeping UTF-16 lengths and indexes as non-canonical runtime caches.
- Apply and invert now operate on span ranges without expanding RichText into
  one allocation per character; complete spans advance in O(1) and partial
  spans reuse one set of calculated text metrics.
- RichText compose and transform now slice inserted text with one UTF-8 boundary
  scan per consumed prefix.
- RichText decoding now enforces snapshot and explicit Change input/output
  logical lengths. Snapshot decoding accepts empty or mergeable text spans and
  normalizes them in memory, while encoding preserves the existing canonical
  wire bytes.
- RichText transform now reports `TransformError::LengthOverflow` instead of
  panicking when a transformed Change length cannot be represented.

## [0.1.0] - 2026-08-08

### Added

- Immutable Null, Bool, Int, Float, String, Text, RichText, List and Map values.
- Canonical Value and Change binary codecs with explicit input limits.
- Snapshot-relative fluent builders for Replace, Map, List, Text, RichText and
  checked Int changes.
- Functional apply, compose, invert and pairwise transform operations.
- Rust-native public API and a synchronous ESM JavaScript/Wasm facade.
- Node.js, Vite, Rollup, browser main-thread, Dedicated Worker and Shared Worker
  package entries without public Wasm initialization.
- Stable JavaScript errors, Change inspection, UTF-16 position conversion and
  explicit resource disposal.

### Compatibility

- Rust MSRV: 1.81 for the published `colla` library.
- Node.js: 22 or newer.
- Bundlers: Vite 5 or newer and Rollup 4 or newer.
- Rust and JavaScript implementations at the same version share canonical
  bytes and OT semantics.
- Patch releases in the 0.1 line preserve public API and wire compatibility.
  A later pre-1.0 minor may include documented breaking changes.

### Known limitations

- Colla provides OT primitives, not Document, Session, history, synchronization,
  transport, presence or editor adapters.
- The core guarantees TP1 but not TP2; consumers must supply an appropriate
  control algorithm.
- The binary codec is a canonical body format. Version envelopes and application
  metadata belong to the consumer.
- CommonJS, Deno, Bun, Service Worker and edge runtimes are not supported in
  0.1.
- RichText embeds are atomic and cannot be edited recursively in place.
- FinalizationRegistry is only a leak-mitigation fallback; applications should
  use `dispose()` or `Symbol.dispose` for deterministic release.
