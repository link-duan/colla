# Colla

Rust-native Operational Transformation primitives for immutable nested values.

The published `colla` library supports Rust 1.81 or newer. Coordinated
JavaScript releases support Node.js 22+, Vite 5+ and Rollup 4+.

The core model is a closed `Value` tree and one recursive, canonical `Change`.
It provides functional Apply, sequential Compose, pairwise Transform (TP1),
Invert, snapshot-aware `ChangeBuilder`, and a strict canonical binary body codec.

    use colla::{apply, path, Value};

    let before = Value::map([("title", Value::text("Draft"))])?;

    let mut builder = before.change();
    builder.text_insert(&path!["title"], 5, " v2")?;
    let change = builder.build();
    let after = apply(&before, &change)?;

Design specifications:

- `docs/data-model.md`
- `docs/ot-properties.md`
- `docs/binary-format.md`
- [`Core roadmap`](docs/roadmap.md)
- [`Release runbook`](docs/releasing.md)
- [`Changelog`](CHANGELOG.md)
- [`@colla/core` JavaScript package](packages/core/README.md)

Run Rust tests with `cargo test --workspace` and JavaScript artifact tests with
`pnpm test:js`.
