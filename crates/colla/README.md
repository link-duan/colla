# colla

`colla` provides immutable nested values, canonical recursive changes, and
Operational Transformation primitives. It is the reference implementation for
the data model and binary format shared with the JavaScript `colla-ot` package.

The crate supports Rust 1.81 or newer.

## Install

```toml
[dependencies]
colla = "0.1"
```

## Values

`Value` is a closed immutable tree containing Null, Bool, Int, Float, String,
Text, RichText, List, and Map values.

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

Ordinary `String` values are atomic and can only be replaced as a whole. `Text`
uses Unicode scalar positions and supports character-level OT. A `Path` is a
Snapshot-relative lookup address; it is not stored in a `Change` and is not
stable across concurrent sequence edits.

Map constructors reject duplicate keys. Floating-point values must be finite;
negative zero is normalized to positive zero.

## Typed Change construction

Rust constructs recursive changes with typed constructors and converts them
with `Into<Change>`. Construction is independent of a Snapshot. Constructors
normalize zero-length operations, empty inserts, adjacent compatible
operations, insert/delete ordering, and trailing retains. Length or allocation
capacity overflow returns `ValueError::LengthOverflow`.

```rust
use colla::{Change, MapChange, MapEntryChange, TextChange, TextOp};

let title: Change = TextChange::from_ops([
    TextOp::Retain(5),
    TextOp::Insert(" v2".into()),
])?
.into();

let change: Change = MapChange::from_entries([(
    "title",
    MapEntryChange::Modify(title),
)])?
.into();

assert!(!change.is_noop());
# Ok::<(), colla::ValueError>(())
```

Use:

- `MapChange::from_entries` for explicit insert, delete, and recursive modify;
- `ListChange::from_ops` for retain, insert, delete, and element modify;
- `TextChange::from_ops` for retain, insert, and delete;
- `RichTextChange::from_ops` for content operations and attribute formatting;
- `IntChange::Add` for checked integer addition;
- `Change::replace` for atomic replacement, including type changes.

An empty typed change and `IntChange::Add(0)` convert to `Change::noop()`.
Unmentioned sequence tails are implicit retains.

## Apply, Compose, Invert, and Transform

```rust
use colla::{
    apply, compose, invert, transform_pair, Change, TextChange, TextOp, TieBreak,
    Value,
};

let base = Value::text("ab");
let first: Change = TextChange::from_ops([
    TextOp::Retain(1),
    TextOp::Insert("x".into()),
])?
.into();
let second: Change = TextChange::from_ops([
    TextOp::Retain(2),
    TextOp::Insert("y".into()),
])?
.into();

let combined = compose(&first, &second)?;
let after = apply(&base, &combined)?;
let inverse = invert(&combined, &base)?;
assert_eq!(apply(&after, &inverse)?, base);

let concurrent: Change = TextChange::from_ops([
    TextOp::Retain(1),
    TextOp::Insert("z".into()),
])?
.into();
let (first_prime, concurrent_prime) =
    transform_pair(&first, &concurrent, TieBreak::LeftFirst)?;
assert_eq!(
    apply(&apply(&base, &first)?, &concurrent_prime)?,
    apply(&apply(&base, &concurrent)?, &first_prime)?,
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`apply` validates Snapshot type, key, and range compatibility. `compose`
combines sequential changes. `invert` requires the original Snapshot because a
Change does not carry old values. `transform_pair` handles two concurrent
changes from one Snapshot and requires a deterministic `TieBreak` for conflicts.

Colla guarantees TP1 for applicable transformed paths. It does not guarantee
TP2; the caller must provide an appropriate control algorithm.

## RichText

RichText is a linear sequence of text and atomic embeds. Text lengths use
Unicode scalar values; every embed has length one. Attributes participate in
OT through explicit Set and Remove changes.

```rust
use colla::{
    apply, AttrChange, AttrPatch, AttrValue, Attrs, Change, RichSpan, RichText,
    RichTextChange, RichTextOp, Value,
};

let base = Value::rich_text(RichText::from_spans(vec![
    RichSpan::text("Hi", Attrs::new()),
    RichSpan::embed(Value::map([("id", Value::string("user-1"))])?, Attrs::new()),
])?);

let patch = AttrPatch::from_entries([
    ("bold", AttrChange::Set(AttrValue::Bool(true))),
])?;
let change: Change = RichTextChange::from_ops([
    RichTextOp::Retain { len: 2, attrs: patch },
])?
.into();

let after = apply(&base, &change)?;
assert_eq!(after.as_rich_text().unwrap().len(), 3);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`RichText::from_spans` removes empty text spans and merges adjacent text spans
with equal attributes. Embeds can be inserted, deleted, or formatted as one
unit, but cannot be recursively edited inside RichText. Use a separate Value
location and a stable reference when embed state must collaborate independently.

`RichText::code_point_to_utf16` and `RichText::utf16_to_code_point` explicitly
convert Snapshot positions for JavaScript or editor integration. UTF-16
positions inside surrogate pairs are rejected.

## Canonical codec and input limits

```rust
use colla::{InputLimits, Value};

let value = Value::text("hello");
let bytes = value.encode();
assert_eq!(Value::decode(&bytes)?, value);

let limits = InputLimits {
    max_string_bytes: 4,
    ..InputLimits::default()
};
assert!(Value::decode_with_limits(&bytes, &limits).is_err());
# Ok::<(), colla::CodecError>(())
```

`Value::encode` and `Change::encode` produce the canonical binary body format.
Decode rejects malformed, trailing, excessive, or non-canonical input.
`InputLimits` are a receiver policy for untrusted input; they do not define the
maximum valid in-memory Value or Change and are not applied to algebra results.

The body format does not include a protocol version, document ID, author,
operation identity, compression, or checksum. Applications must provide their
own envelope when those fields are required.

## Errors

Construction errors use `ValueError`. Algebra exposes `ApplyError`,
`ComposeError`, `InvertError`, and `TransformError`. Codec failures use
`CodecError`, while explicit UTF-16 conversion uses `Utf16PositionError`.

Errors are structured and should be matched by variant. Error messages are for
humans and are not a stable machine-readable protocol.

## Examples

The crate includes executable examples:

```sh
cargo run -p colla --example basic_edit
cargo run -p colla --example collab_demo
cargo run -p colla --example binary_roundtrip
```

## Scope

Colla provides foundational OT values, changes, algebra, and codecs. It does
not provide a Document or Session abstraction, history, synchronization,
transport, presence, cursors, persistence envelopes, or editor adapters.

## More documentation

- [Rust API reference](https://docs.rs/colla)
- [JavaScript guide](https://github.com/link-duan/colla/blob/master/packages/core/README.md)
- [Core data model](https://github.com/link-duan/colla/blob/master/docs/data-model.md)
- [OT properties](https://github.com/link-duan/colla/blob/master/docs/ot-properties.md)
- [Canonical binary format](https://github.com/link-duan/colla/blob/master/docs/binary-format.md)
- [Changelog](https://github.com/link-duan/colla/blob/master/CHANGELOG.md)
