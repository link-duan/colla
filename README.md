# Colla

Rust-native Operational Transformation primitives for immutable nested values.

The core model is a closed `Value` tree and one recursive, canonical `Change`.
It provides functional Apply, sequential Compose, pairwise Transform (TP1),
Invert, snapshot-aware `ChangeBuilder`, and a strict canonical binary body codec.

    use colla::{path, ChangeBuilder, Limits, Value};

    let limits = Limits::default();
    let before = Value::map([("title", Value::text("Draft"))])?;

    let mut builder = ChangeBuilder::new(&before, &limits)?;
    builder.text_insert(&path!["title"], 5, " v2")?;
    let change = builder.build();
    let after = change.apply_to(&before, &limits)?;

Design specifications:

- `docs/data-model.md`
- `docs/ot-properties.md`
- `docs/binary-format.md`

Run tests with `cargo test`.
