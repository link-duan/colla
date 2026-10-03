# Changelog

All notable public changes to the Rust `colla` crate and the `colla-ot`
package are recorded here. Both artifacts always use the same version.

## [Unreleased] — 0.4.0

0.4 replaces the 0.3 Change primitives with a Rust-owned editing, history and
synchronization runtime. None of the 0.3 APIs or bytes carry over.

### Changed

- **BREAKING (Change model).** The recursive typed Change tree (`ChangeKind`,
  `MapChange`/`ListChange`/`IntChange` with nested Modify, `Change.fromJS` and
  `Change.build`) is replaced by an ordered sequence of path-addressed operations:
  Insert, Delete, Set, ListMove, Text, Add and RichText. Each operation's Path
  and positions refer to the content produced by the preceding operations.
- **BREAKING (algebra).** Every function takes an explicit base Value:
  `apply(base, change)`, `invert(base, change)`, `compose(base, first, second)`
  and `transform(base, left, right, priority)`. `transformPair`/`transform_pair`
  and `TieBreak` are replaced by `transform` and `Priority`; transform returns
  left-after-right then right-after-left and always produces a result (TP1).
- **BREAKING (JavaScript Value).** The Wasm-backed `Value` handle with `clone()`,
  `dispose()` and facade finalizers becomes an immutable `Value` object; GC owns
  all Wasm memory and no handle is exposed. `ValueInput`/`ValueData` become
  `Input`, and Text/RichText are immutable `Text`/`RichText` objects created by
  `text()`/`richText()` instead of plain `{ type }` data. The RichText
  discriminator is `"richtext"` instead of `"richText"`.
- **BREAKING (Rust Value).** `Value` exposes a `Body` enum, `get(&[Segment])` and
  structural equality instead of the `List`/`Map`/`Text`/`FiniteF64`/`ValueKind`
  types; `Path`/`PathSeg` become `Path`/`Segment`.
- **BREAKING (errors).** `ValueError`, `ApplyError`, `ComposeError`,
  `InvertError`, `TransformError`, `CodecError` and `Utf16PositionError` are
  replaced by one `CollaError` with a stable `code`, `operation` and string
  `details`, shared by Rust and JavaScript.
- **BREAKING (wire format).** Every encoded object starts with a typed envelope
  (`COLLA`, codec version, object kind). Values and Changes use new layouts, and
  protocol records are cocodec derived structs that omit trailing default fields.
  Bytes produced by 0.3 do not decode.
- **BREAKING (limits).** `InputLimits`, `DEFAULT_INPUT_LIMITS` and per-call
  input options are removed. All construction, editing and decoding enforce
  fixed limits: depth 100, 1,000,000 nodes and 16 MiB per string.
- The Rust API is exported from the crate root instead of public modules.

### Added

- Document: a Rust-implemented runtime exposed in both languages. `edit` runs a
  synchronous Transaction with scoped List/Text/RichText editors (UTF-16
  positions in JavaScript) and returns an EditResult (before, after, change,
  inverse, local version, origin); `subscribe` delivers isolated events.
- ListMove: move an element within its List; concurrent edits inside it follow.
- History: undo/redo with explicit groups, rebasing across remote edits, and
  HistoryCheckpoint restore.
- Centralized synchronization: SyncSnapshot, Submission and ServerMessage
  (Commit or Rejection); a client `SyncSession` with one in-flight request plus
  a composed buffer; and an immutable server `Authority` that orders commits,
  deduplicates retries, reports revision gaps and compacts history. Rejections
  and unrecoverable rebases enter recovery-required while retaining local work.
- SessionCheckpoint and AuthorityCheckpoint for complete client and server
  restart, alongside Value and SyncSnapshot persistence.

### Removed

- `inspectChange` and Change View, `resolveCodePointPosition`/`resolveUtf16Position`
  and the `int()` helper.
- The Rust `json` feature. The `__bindings` feature exists only for the private
  binding crate and is exempt from semver.
- The Wasm `serde_json` runtime dependency.

### Documentation

- Rebuilt the site around Core, Editing, History and Sync, with dedicated examples,
  production guides and API references. Old documentation routes are removed without
  redirects. Topic, link and anchor checks plus executable examples guard against
  content regressions.

### Validation

- Property and randomized TP1 tests for path transformation, shared Rust/JS golden
  fixtures, three-client restart and retry simulation, malformed-input fuzzing,
  package installation, browser/Worker/bundler and memory tests.
- All guides, examples and package versions target the unreleased 0.4.0 development
  version. Actual publication is separate from this implementation.

## [0.3.0] - 2026-08-25

### Changed

- **BREAKING (wire format).** Adopted [`cocodec`](https://crates.io/crates/cocodec)
  as the canonical binary codec. Value/Change tags were renumbered (`Bool` is no
  longer a two-tag hack; `Int`/`String`/`Text`/`RichText`/`List`/`Map` shift down
  by one) and byte layouts changed, so bytes produced by 0.2.x no longer decode.
  colla is early-stage with no external consumers, so the break is taken now.
- **Decoding is now structural.** Byte decoding no longer enforces _semantic_
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

- Added a stable `ErrorCode` classification to the Rust `colla` crate: a
  `#[non_exhaustive]` `ErrorCode` enum with `as_str()` and `ALL`, plus a `code()`
  accessor on `ValueError`, `ApplyError`, `ComposeError`, `TransformError`,
  `InvertError`, and `CodecError`. This is the single source of truth for error
  codes across the WebAssembly facade, the Rust golden tests, and the
  `colla-ot` `ErrorCode` union type (now the type of `CollaError.code`).

### Fixed

- `Value.fromJS` now enforces `maxSequenceLength` on richText values. A crafted
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
