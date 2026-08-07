# Changelog

All notable public changes to the Rust `colla` crate and the `@colla/core`
package are recorded here. Both artifacts always use the same version.

## [0.1.0] - Unreleased

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
