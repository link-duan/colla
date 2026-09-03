---
title: Rust API reference
description: Public modules, types, OT operations, and codecs in the colla crate.
---

# Rust API

<p class="eyebrow">Reference</p>
<p class="lead">The <code>colla</code> crate is the reference implementation of Colla's immutable data model, OT algebra, and canonical Core codec. It gives Rust applications the primitives; it does not prescribe a server or session runtime.</p>

For a guided first edit, read the [Rust examples](/docs/examples/rust). For every
public symbol, trait implementation, and exact signature, use the generated
[API documentation on docs.rs](https://docs.rs/colla).

## Install

Add the crate to `Cargo.toml`:

```toml
[dependencies]
colla = "0.3"
```

The crate uses Rust 2021 edition and supports Rust 1.81 or newer. The published
crate and the JavaScript `colla-ot` package are built from the same model and
golden codec fixtures.

## Public module map

| Module | What it contains |
| --- | --- |
| `value` | Immutable `Value`, `Text`, `List`, `Map`, and finite floats. |
| `change` | Recursive `Change` plus typed Map/List/Text/RichText operations. |
| `op` | `apply`, `compose`, `invert`, and `transform_pair`. |
| `richtext` | Spans, atomic embeds, attributes, and coordinate conversion. |
| `attrs` | Canonical attribute sets and formatting patches. |
| `document` | `Snapshot` and `Update` local envelopes. |
| `codec` | Canonical Value and Change body encoding/decoding. |
| `error` | Structured errors and the shared `ErrorCode` taxonomy. |
| `input_limits` | Receiver-defined limits for structured untrusted input. |
| `path` | Snapshot-relative map keys and list indexes. |

The crate root re-exports the types most applications need:

```rust
use colla::{
    apply, compose, invert, transform_pair, AttrChange, AttrPatch, AttrValue,
    Attrs, Change, ChangeKind, ErrorCode, InputLimits, List, ListChange,
    ListOp, Map, MapChange, MapEntryChange, RichContent, RichSpan, RichText,
    RichTextChange, RichTextOp, Snapshot, Text, TextChange, TextOp, TieBreak,
    Update, Value, ValueKind, ValueType,
};
```

## Values

`Value` is a closed, immutable tree with structural sharing. Its variants are
`Null`, `Bool`, signed `Int(i64)`, finite `Float`, atomic `String`, collaborative
`Text`, `RichText`, ordered `List`, and string-keyed `Map`.

```rust
use colla::{path, Value};

let value = Value::map([
    ("title", Value::text("Draft")),
    ("tags", Value::list([Value::string("ot"), Value::string("rust")])),
])?;

assert_eq!(
    value.get(&path!["title"]).and_then(Value::as_text).unwrap().as_str(),
    "Draft",
);
# Ok::<(), colla::ValueError>(())
```

| Type | Construction | Key behavior |
| --- | --- | --- |
| `Value::string` | `Value::string("label")` | Atomic replacement; no character-level OT. |
| `Value::text` | `Value::text("editable")` | Unicode scalar positions and text OT. |
| `Value::rich_text` | `Value::rich_text(rich_text)` | Text spans and one-unit atomic embeds. |
| `Value::list` | `Value::list(iter)` | Ordered nested Values. |
| `Value::map` | `Value::map(entries)` | Duplicate keys return `ValueError::DuplicateKey`. |
| `Value::float` | `Value::float(number)` | Rejects NaN/infinity; normalizes negative zero. |

`Value` implements standard `From<T>` conversions and provides typed accessors
`as_bool()`, `as_int()`, `as_float()`, `as_finite_float()`, `as_string()`, `as_text()`,
`as_rich_text()`, `as_list()`, `as_map()`, and `is_null()`. `Path` is a temporary lookup
address built fluently with `with_key` and `with_index`; it is not stored in a Change
or codec body.

## Changes and typed constructors

`Change` is a canonical recursive operation relative to a base Value. Creation
does not inspect a Snapshot; compatibility is checked by `apply`.

```rust
use colla::{Change, MapChange, MapEntryChange, TextChange, TextOp};

let title: Change = TextChange::from_ops([
    TextOp::Retain(5),
    TextOp::Insert(" v2".into()),
])?.into();

let change: Change = MapChange::from_entries([
    ("title", MapEntryChange::Modify(title)),
])?.into();

assert!(!change.is_noop());
# Ok::<(), colla::ValueError>(())
```

| Constructor | Operation stream |
| --- | --- |
| `MapChange::from_entries` | Unique key entries: `Insert`, `Delete`, or recursive `Modify`. |
| `ListChange::from_ops` | `Retain`, `Insert`, `Delete`, and one-element `Modify`. |
| `TextChange::from_ops` | `Retain`, `Insert`, and `Delete` Unicode scalars. |
| `RichTextChange::from_ops` | Content operations plus attribute patches. |
| `IntChange::Add(delta)` | Checked signed 64-bit integer addition. |
| `Change::replace(value)` | Atomic replacement, including a type change. |

Typed constructors normalize zero-length operations, empty inserts, adjacent
compatible operations, insert/delete ordering, and trailing plain retains.
Empty typed changes and `IntChange::Add(0)` convert to `Change::noop()`. `Change`
provides `From<Value>` (equivalent to `Change::replace`) and typed extractors
`as_replace()`, `as_map()`, `as_list()`, `as_text()`, `as_rich_text()`, and `as_int()`.
The canonical `Change` exposes `kind()` and `is_noop()`; it does not contain old
values, versions, authorship, or operation IDs.

## OT operations

```rust
use colla::{apply, compose, invert, transform_pair, Change, TextChange, TextOp, TieBreak, Value};

let base = Value::text("ab");
let first: Change = TextChange::from_ops([
    TextOp::Retain(1), TextOp::Insert("x".into()),
])?.into();
let second: Change = TextChange::from_ops([
    TextOp::Retain(2), TextOp::Insert("y".into()),
])?.into();

let combined = compose(&first, &second)?;
let after = apply(&base, &combined)?;
let inverse = invert(&combined, &base)?;
assert_eq!(apply(&after, &inverse)?, base);

let (first_prime, second_prime) =
    transform_pair(&first, &second, TieBreak::LeftFirst)?;
assert_eq!(
    apply(&apply(&base, &first)?, &second_prime)?,
    apply(&apply(&base, &second)?, &first_prime)?,
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

| Function | Contract |
| --- | --- |
| `apply(base, change)` | Return a new Value; reject type, key, range, and integer incompatibilities without mutating `base`. |
| `compose(first, second)` | Produce one operation equivalent to applying `first` then `second`. |
| `invert(change, base)` | Build an inverse; the original base is required because Change carries no old values. |
| `transform_pair(left, right, tie_break)` | Transform concurrent Changes from one base using `TieBreak::LeftFirst` or `RightFirst`. |

Colla guarantees the pairwise TP1 convergence equation for applicable
transforms. It does not guarantee TP2 path independence; a distributed control
algorithm must provide the required context and ordering.

## RichText and coordinates

`RichText` is a canonical sequence of text spans and atomic embed spans. Text
lengths use Unicode scalar values; each embed counts as one. `Attrs` contains
atomic Bool, Int, Float, or String values in canonical key order. Formatting uses
`AttrPatch` with explicit `Set` and `Remove` actions; `Null` is not a deletion
sentinel.

```rust
use colla::{Attrs, RichSpan, RichText, Value};

let rich = RichText::from_spans(vec![
    RichSpan::text("Hello", Attrs::new()),
    RichSpan::embed(Value::int(1), Attrs::new()),
])?;
assert_eq!(rich.len(), 6); // five scalars plus one embed
# Ok::<(), colla::ValueError>(())
```

`RichText::from_spans` removes empty text spans and merges adjacent text spans
with equal attributes. `Text` and `RichText` provide `code_point_to_utf16` and
`utf16_to_code_point` to convert positions for JavaScript/editor boundaries; positions
inside a surrogate pair return `Utf16PositionError::InvalidUtf16Boundary`.
`Utf16PositionError` implements `.code() -> ErrorCode`.

## Snapshots, Updates, and codecs

The crate's `Snapshot` and `Update` types are envelopes, not a Document state
machine:

```rust
use colla::{Change, Snapshot, Update, Value};

let snapshot = Snapshot::new(7, Value::text("Draft"));
let snapshot_bytes = snapshot.encode();
let restored = Snapshot::decode(&snapshot_bytes)?;
assert_eq!(restored.revision(), 7);

let update = Update::new(7, 3, Change::noop());
let restored_update = Update::decode(&update.encode())?;
assert_eq!(restored_update.update_id(), 3);
# Ok::<(), colla::CodecError>(())
```

`Value::encode`/`decode`, `Change::encode`/`decode`, and the equivalent
`codec::encode_*` functions operate on canonical, versionless Core bodies.
`Snapshot` uses `COLLAS`; `Update` uses `COLLAU`; both currently use protocol
version `1`. See [Protocol reference](/reference/protocol) for byte layouts and
strict decoding behavior.

## Errors and input policy

Errors are typed and `#[non_exhaustive]`; match variants or their stable
`ErrorCode` classification rather than display strings.

| Error family | Typical causes |
| --- | --- |
| `ValueError` | Non-finite float, duplicate key, platform length overflow. |
| `ApplyError` | Type/key/index mismatch, sequence bounds, integer overflow. |
| `ComposeError` | Incompatible sequential kinds or composed length overflow. |
| `InvertError` | Change is not applicable to the supplied base. |
| `TransformError` | Changes cannot share one valid base or exceed limits. |
| `CodecError` | Invalid bytes, envelope magic/version, unknown tag, or trailing data. |
| `Utf16PositionError` | Out-of-range or surrogate-splitting coordinate. |

`ErrorCode::as_str()` exposes the stable strings such as `invalid_encoding`,
`type_mismatch`, `incompatible_change`, and `limit_exceeded`. `InputLimits`
describes receiver policy for structured input (`max_depth`, node counts,
container and string sizes, sequence operation count, and sequence length). The
canonical byte decoder uses cocodec's built-in depth/allocation defenses and
does not accept configurable limits; algebra results are not restricted by
`InputLimits`.

## What this crate does not provide

The Rust crate provides values, changes, algebra, and codecs. It does not
provide `Document`/`Session` state, transport, server
ordering, history, presence, cursors, persistence storage, authentication, or
editor adapters. Add those policies around the primitives. The JavaScript
package's [Document state](/reference/javascript) supplies local visible state
and Snapshot/Update queue behavior when that is the appropriate boundary.

## More documentation

- [Full API on docs.rs](https://docs.rs/colla)
- [Rust examples](/docs/examples/rust)
- [Data model](/docs/core/values)
- [OT guide](/docs/ot/)
- [Protocol reference](/reference/protocol)
- [JavaScript API](/reference/javascript)
- [Rust crate source](https://github.com/link-duan/colla/tree/master/crates/colla)
