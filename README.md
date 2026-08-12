# Colla

Colla provides Rust-native Operational Transformation primitives for immutable,
nested values. The same data model, canonical binary format, and OT semantics
are exposed to JavaScript through the synchronous `colla-ot` package.

Colla includes:

- immutable `Value` trees and recursive, canonical `Change` values;
- Apply, Compose, Invert, and pairwise Transform with TP1 guarantees;
- collaborative Text and RichText with atomic embeds;
- typed Rust and JavaScript Change construction;
- a strict canonical binary body codec with configurable input limits.

Colla is not a document runtime. It does not provide sessions, history,
synchronization, transport, presence, cursors, or editor-specific formats.

## Rust

```rust
use colla::{apply, Change, MapChange, MapEntryChange, TextChange, TextOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let before = Value::map([("title", Value::text("Draft"))])?;
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
    let _after = apply(&before, &change)?;
    Ok(())
}
```

The published `colla` crate supports Rust 1.81 or newer.

## JavaScript

```ts
import { Change, Value, apply, text } from "colla-ot"

using before = Value.fromJS({ title: text("Draft") })
using change = Change.build(change => {
  change.map(map => {
    map.modify("title", title => {
      title.text(text => text.retain(5).insert(" v2"))
    })
  })
})
using after = apply(before, change)
```

`colla-ot` supports Node.js 22+, Vite 5+, and Rollup 4+. Text and RichText
Change construction uses Unicode scalar lengths; explicit helpers convert
between those positions and JavaScript UTF-16 positions when a Snapshot is
available.

## Documentation

- [Rust guide](crates/colla/README.md)
- [JavaScript guide](packages/core/README.md)
- [Rust API reference](https://docs.rs/colla)
- [Documentation index](docs/README.md)

The documentation index also links the normative specifications, architecture
decisions, roadmap, release runbook, domain language, and changelog.

Run Rust tests with `cargo test --workspace --all-targets` and JavaScript
artifact tests with `pnpm test:js`.
