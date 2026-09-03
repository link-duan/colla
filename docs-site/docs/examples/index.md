# Examples overview

Examples cover minimal Value editing, binary round-trips, concurrent transforms, and optimistic JavaScript Document synchronization. They are runnable usage guides; the host application must still provide protocol ordering, retries, and persistence.

## Rust examples

- [`basic_edit.rs`](https://github.com/link-duan/colla/blob/master/crates/colla/examples/basic_edit.rs): Modify Text in a nested Map.
- [`binary_roundtrip.rs`](https://github.com/link-duan/colla/blob/master/crates/colla/examples/binary_roundtrip.rs): Encode and decode a Value.
- [`collab_demo.rs`](https://github.com/link-duan/colla/blob/master/crates/colla/examples/collab_demo.rs): Verify TP1 for two concurrent Text Changes.

## JavaScript examples

- [Document local/remote synchronization](./javascript-document)
- [Core Values and Changes](./javascript-core)

Run a Rust example with `cargo run -p colla --example <name>`.
Build `packages/core` before running JavaScript examples.
The JavaScript package requires Node.js 22 or newer.

## What the examples demonstrate

The examples use explicit `text()` markers for collaborative text.
They keep ordinary strings atomic.
They construct Changes independently from the base Value.
They use `apply` to produce a new immutable Value.
They encode and decode canonical bytes at explicit boundaries.
They use `left-first` only where concurrent order is ambiguous.
They acknowledge local updates in FIFO order.
They keep retry and transport policy outside Colla.

## Reading the examples

Start with the Core example if you are learning Value and Change syntax.
Continue with the Document example when you need optimistic visible state.
Use the Rust example to compare the shared model across languages.
Use the round-trip example to understand codec ownership.
Use the collaboration example to see the TP1 assertion.

None of these examples authenticates a request or persists a session.
Those responsibilities belong to the host application.

## Next

Read [JavaScript Document](./javascript-document) for an end-to-end local/remote flow.
Then read [Immutable Value and Change](./javascript-core) or [Rust](./rust).
For production concerns, continue to [production integration](../production/).
