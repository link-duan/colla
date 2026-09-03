---
title: Glossary, errors, and limits
description: Shared Colla terminology, error codes, coordinate rules, and input policy.
---

# Glossary, errors, and limits

<p class="eyebrow">Reference</p>
<p class="lead">Use the same words at the Core, Document, and protocol boundaries. This page is the quick lookup for terms, stable error classifications, coordinate conventions, and resource policy.</p>

For operation semantics, see the [Data model](/docs/core/values) and the [OT
guide](/docs/ot/). For byte-level details, see the [Protocol
reference](/reference/protocol).

## Data model

| Term | Meaning |
| --- | --- |
| **Value** | An immutable, closed recursive tree: Null, Bool, Int, Float, String, Text, RichText, List, or Map. |
| **Core Value** | An immutable Value used by the Rust `colla` crate and JavaScript `colla-ot` package. |
| **String** | An atomic UTF-8 value replaced as a whole; it is not character-level OT. |
| **Text** | A collaborative sequence whose operation lengths count Unicode scalar values. |
| **RichText** | Text and atomic embed spans with canonical attributes and formatting patches. |
| **Embed** | One atomic Value inside RichText; it counts as one sequence unit and cannot be recursively edited there. |
| **List** | An ordered sequence of nested Values. List `Modify` consumes one base element. |
| **Map** | A string-keyed collection with unique keys and canonical lexical ordering. |
| **Attrs** | RichText formatting attributes; values are Bool, Int, finite Float, or String. |
| **AttrPatch** | Explicit `Set`/`Remove` changes applied to retained RichText content. |
| **Path** | A temporary Snapshot-relative sequence of map keys and list indexes; never part of a Change or body. |

## Operations and state

| Term | Meaning |
| --- | --- |
| **Change** | One immutable forward operation relative to a base Value. It carries no old values, revision, author, or ID. |
| **Noop** | The identity Change. Empty typed changes and `IntAdd(0)` normalize to Noop. |
| **Replace** | Replaces a complete target Value, including its type. |
| **Apply** | Executes a Change against a concrete base and returns a new Value. |
| **Compose** | Combines sequential Changes so the result has the same effect as applying them in order. |
| **Invert** | Builds an undo Change; the original base is required because a Change has no old values. |
| **Transform** | Rewrites concurrent Changes from one common base. Colla exposes pairwise `transform_pair`/`transformPair`. |
| **TieBreak** | Deterministic `LeftFirst`/`RightFirst` ordering for otherwise unresolved concurrent conflicts. |
| **TP1** | Pairwise convergence: applying transformed operations in either order yields the same result when both are applicable. |
| **TP2** | Path independence across three or more transform paths; Colla does not guarantee it. |
| **Document** | JavaScript mutable state: visible Value, confirmed revision, pending Updates, and events. |
| **Snapshot** | A persistable content checkpoint containing revision and visible Value. |
| **Update** | One Change plus its base revision and local `updateId`, suitable for an outer application transport. |
| **Confirmed revision** | The latest server-ordered revision incorporated into a Document's confirmed state. |
| **Pending update** | A local Update applied optimistically but not yet acknowledged in FIFO order. |
| **Rebase** | Transforming pending local Changes over an accepted remote Update while preserving visible edits. |

## Codec vocabulary

| Term | Meaning |
| --- | --- |
| **Core body** | A versionless canonical Value or Change byte sequence produced by `encode`. |
| **Canonical** | A unique byte representation plus constructor-normalized semantic operation form. |
| **cocodec** | The shared low-level codec machinery for tags, varints, UTF-8, ordering, and decoder defenses. |
| **Envelope** | A versioned wrapper around a body. `COLLAS` wraps Snapshot data; `COLLAU` wraps Update data. |
| **Magic** | The six ASCII bytes identifying an envelope type: `COLLAS` or `COLLAU`. |
| **Protocol version** | The little-endian `u16` in an envelope header; the current version is `1`. |
| **Trailing bytes** | Any bytes left after one complete value, Change, Snapshot, or Update; strict decoders reject them. |
| **Byte canonicality** | Structural wire rules such as shortest varints, valid UTF-8, known tags, key order, and complete input. |
| **Semantic normalization** | Constructor/algebra cleanup such as merging adjacent operations and dropping empty retains. |

Snapshot and Update envelopes are not interchangeable with raw Value and Change
bodies. A Snapshot payload is `(revision, content)`; an Update payload is
`(revision, updateId, change)`. See [Protocol reference](/reference/protocol).

## Stable error codes

The JavaScript facade throws `CollaError` with a stable string code. Rust keeps
rich typed error families and exposes `.code()`/`ErrorCode::as_str()` for the
families that participate in the shared taxonomy.

| Code | Meaning | Common source |
| --- | --- | --- |
| `invalid_encoding` | Input bytes are malformed, unsupported, non-canonical, or incomplete. | `Value.decode`, `Change.decode`, `Snapshot.decode`, `Update.decode`; Rust `CodecError`. |
| `limit_exceeded` | A configured or built-in resource/length limit was exceeded. | Structured input, sequence arithmetic, or decoder depth. |
| `type_mismatch` | Change kind does not match the target Value kind. | `apply`. |
| `missing_key` | A Map delete/modify targeted an absent key. | `apply`. |
| `key_already_exists` | A Map insert targeted an existing key. | `apply`. |
| `out_of_bounds` | A list index or sequence range is outside the base. | `apply`; JavaScript coordinate conversion also uses this classification. |
| `integer_overflow` | Checked signed integer addition overflowed. | `IntChange::Add` / `intAdd`. |
| `incompatible_change` | Sequential or concurrent Changes cannot share the required context. | `compose`, `transformPair`, or remote revision checks. |
| `invalid_value` | Value construction violated a model invariant. | Non-finite float, duplicate key, invalid attribute/value. |
| `invalid_state` | A JavaScript resource was disposed or a lifecycle operation is no longer valid. | `Document`, `Snapshot`, `Update`, `ValueHandle`, or `Change`. |
| `invalid_argument` | JavaScript facade received the wrong runtime type or option shape. | Public JS constructors and methods. |
| `invalid_utf16_boundary` | A JavaScript/editor position splits a UTF-16 surrogate pair. | JavaScript `resolveUtf16Position` and projections; Rust uses `Utf16PositionError::InvalidUtf16Boundary`. |

The first ten codes (including `invalid_utf16_boundary`) are shared with the Rust
crate. `invalid_state` and `invalid_argument` are additionally surfaced by the
JavaScript facade. Human-readable error messages and detailed paths are diagnostics;
branch on the stable code.

## Structured input limits

JavaScript `InputOptions` can override these defaults for `fromJS` and builder
input. Rust's `InputLimits` names the same receiver-policy fields.

| Limit | JavaScript field | Default | Counts |
| --- | --- | ---: | --- |
| Depth | `maxDepth` / `max_depth` | `128` | Recursive Value and Change nesting. |
| Value nodes | `maxValueNodes` / `max_value_nodes` | `1,000,000` | Raw Value nodes. |
| Change nodes | `maxChangeNodes` / `max_change_nodes` | `1,000,000` | Raw Change nodes. |
| Container length | `maxContainerLength` / `max_container_len` | `1,000,000` | Map, List, and attribute entries. |
| String bytes | `maxStringBytes` / `max_string_bytes` | `16 MiB` | One UTF-8 string. |
| Sequence operations | `maxSequenceOps` / `max_sequence_ops` | `1,000,000` | Raw operation count before normalization. |
| Sequence length | `maxSequenceLength` / `max_sequence_len` | `1,000,000` | Logical input/output sequence length. |

These are receiver policy, not a change to the Value type's semantics. They are
checked on structured input before normalization. Canonical byte decoding relies
on cocodec's fixed recursion and safe length handling; algebra results and
editor projections do not consume `InputLimits`.

## Coordinates and ownership

- Core Text and RichText operations use Unicode scalar positions.
- JavaScript editor projections use Snapshot-relative UTF-16 positions.
- A UTF-16 index inside a surrogate pair is invalid; do not silently round it.
- RichText embeds count as one unit in both coordinate systems.
- `ValueHandle`, `Change`, `Snapshot`, and `Update` own Wasm-backed resources in JavaScript; clones own independent resources.
- Returned JavaScript values, event payloads, paths, and edit steps are recursively frozen.
- `Document` disposal is idempotent; using a disposed resource reports `invalid_state`.

## Quick lookup

- [JavaScript API](/reference/javascript) — imports, methods, limits, and lifecycle.
- [Rust API](/reference/rust) — crate modules, typed constructors, and error families.
- [Protocol reference](/reference/protocol) — tags, envelopes, and strict decoding.
- [Getting started](/docs/getting-started) — first application path.
- [Document state](/docs/document/) — local/remote state and persistence.
