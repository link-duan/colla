# Changelog

All notable public changes to the Rust `colla` crate and the `colla-ot`
package are recorded here. Both artifacts always use the same version.

## [Unreleased]

### Changed

- **BREAKING (JavaScript resource lifecycle).** Wasm-backed objects now rely on
  JavaScript GC and wasm-bindgen-generated finalizers by default. The optional
  `dispose()`/`Symbol.dispose` APIs remain available for deterministic cleanup;
  handwritten facade-level `FinalizationRegistry` registrations were removed.
- **BREAKING (JavaScript package exports).** The `colla-ot` package now exposes
  `Document`, `Snapshot`, `Update`, immutable Values and Changes, codecs, and OT
  operations from one package root. The `colla-ot/core` subpath is removed
  without a compatibility alias; replace imports from `colla-ot/core` with
  imports from `colla-ot`.
- **BREAKING (JavaScript API).** `Document.subscribe()` is replaced by typed
  `on("change", listener)` and `on("error", listener)` subscriptions. Change
  events expose edit steps instead of an owned Core Change handle, and listener
  failures no longer fail the committed operation.
- **BREAKING (JavaScript API).** Unified every RichText discriminator to
  `"richtext"`: `ValueKind`, `RichText.type`, `ChangeInput.type`, Change View
  entry types, Edit Steps, and Wasm error details. The former `"richText"`
  discriminator is removed without a compatibility alias. TypeScript/Rust
  symbol names such as `RichText`, `richText()`, and `.richText()` are unchanged.
- **BREAKING (JavaScript API).** Renamed the Wasm-backed `Value` class to
  `ValueHandle`. Unified `ValueInput` and `ValueData` as the recursive `Value`
  type, together with the corresponding `ValueMap`, `Text`, `RichTextSpan`, and
  `RichText` types. No compatibility aliases are provided: replace
  `Value.fromJS`/`Value.decode` with `ValueHandle.fromJS`/`ValueHandle.decode`
  and use `Value` for both structured input and materialized output.
  `CollaError.details.reason` now uses the unified terminology as well (for
  example, `cyclic Value` and `unsupported Value`).
- **BREAKING (wire format).** Adopted [`cocodec`](https://crates.io/crates/cocodec)
  as the canonical binary codec. Value/Change tags were renumbered (`Bool` is no
  longer a two-tag hack; `Int`/`String`/`Text`/`RichText`/`List`/`Map` shift down
  by one) and byte layouts changed, so bytes produced by 0.2.x no longer decode.
  colla is early-stage with no external consumers, so the break is taken now.
- **Decoding is now structural.** Byte decoding no longer enforces *semantic*
  canonicalization (zero-length ops, empty inserts, mergeable adjacent ops,
  `Modify(Noop)`, trailing retains, negative zero). These are the job of the
  construction APIs (`from_ops`/`from_entries`/`from_spans`) and `normalize`.
  `-0.0` is normalized to `+0.0` on decode; RichText still merges adjacent
  equal-attribute spans via `from_spans`. Byte-canonical rules (minimal varint,
  UTF-8, unknown tags, ordering, trailing bytes) are still enforced.
- Unified the `limit_exceeded` `details.limit` names into a stable, bijective
  set (one name per `InputLimits` field). These now apply only to the structured
  `fromJS` path (see Removed).
- Collapsed the canonical binary codec to a single implementation. The
  `colla-ot` facade no longer contains a hand-written byte encoder/decoder; it
  now marshals structured values across the WebAssembly boundary and the Rust
  `colla` codec is the sole implementation of the wire format. Observable
  behavior (canonical bytes, `toJS` shapes, error codes) is unchanged.

### Removed

- **BREAKING.** Byte decoding no longer accepts `InputLimits`. `decode_with_limits`
  is removed and `Value::decode`/`Change::decode` (and the wasm/JS `decode`) take
  no limits argument; decoding is bounded solely by cocodec's built-in defenses
  (a fixed recursion depth and no pre-allocation from untrusted lengths).
  `InputLimits` now bounds only the structured `fromJS` input path.

### Added

- Added `Document` local/remote update handling with optimistic local edits,
  pending rebase, acknowledgements, typed events, and content snapshots.
- Added versioned `Snapshot` and `Update` envelopes with `COLLAS`/`COLLAU`
  magic headers and protocol version 1, including Rust and JavaScript codecs.
- Simplified the local Snapshot/Update payloads to direct cocodec tuples. This
  is an early-development format change with no historical byte compatibility
  promise.
- Root and the JavaScript package's public imports use one runtime-specific
  Wasm initialization module.
- Added `convertChangeToEditSteps(change, base)`, a recursively frozen,
  Snapshot-relative projection that preserves Map/List/Text/RichText operation
  boundaries. Text and RichText consumed lengths are exposed as UTF-16 code
  units, while List `modify.steps` are relative to the current element root.
- Added a stable `ErrorCode` classification to the Rust `colla` crate: a
  `#[non_exhaustive]` `ErrorCode` enum with `as_str()` and `ALL`, plus a `code()`
  accessor on `ValueError`, `ApplyError`, `ComposeError`, `TransformError`,
  `InvertError`, and `CodecError`. This is the single source of truth for error
  codes across the WebAssembly facade, the Rust golden tests, and the
  `colla-ot` `ErrorCode` union type (now the type of `CollaError.code`).

### Fixed

- `ValueHandle.fromJS` now enforces `maxSequenceLength` on richText values. A crafted
  input with few spans of near-`maxStringBytes` text could previously drive the
  total length past a caller-configured `maxSequenceLength` undetected; it is
  now rejected with `limit_exceeded` (`sequence length`).

## [0.2.0] - 2026-08-12

### Changed

- Renamed Rust `RichInsert` to `RichContent` without a compatibility alias,
  made `RichSpan` construction controlled, and replaced the exposed span slice
  with `iter_spans()` and `span_count()`.
- Reworked RichText around canonical spans with cached scalar/UTF-16 metrics and
  cumulative indexes. Apply and invert now process span ranges without
  per-character intermediate arrays; compose and transform avoid repeated
  UTF-8 prefix scans.
- Replaced Rust Change builders and legacy constructors with fallible typed
  `MapChange::from_entries` and sequence `from_ops` constructors plus standard
  `Into<Change>` conversions. Empty typed changes and `IntChange::Add(0)` become
  Noop.
- Added checked Change input/output length accumulation and `LengthOverflow`
  propagation through construction, compose, transform, and invert.
- Replaced JavaScript `Value.change()` and the Snapshot-aware Wasm Builder with
  `Change.fromJS(ChangeInput)` and a pure TypeScript `Change.build()` callback
  builder. Construction is Snapshot-independent and map insert/delete/modify
  semantics are explicit.
- Unified Rust and JavaScript Change construction on Unicode scalar sequence
  lengths. Snapshot-relative Change View and explicit coordinate conversion
  continue to expose UTF-16 positions for JavaScript consumers.
- Applied `InputLimits` to raw JavaScript Change input before normalization so
  empty or mergeable operations cannot bypass resource limits.
- Kept canonical wire bytes unchanged. RichText Snapshot decoding accepts and
  normalizes empty or mergeable Text spans for compatibility, while Change
  decoding remains strict.

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
