# Rust examples

The repository includes runnable examples for Core values, codecs, and OT.
They are executable references rather than a complete collaboration server.

## Basic nested edit

`basic_edit.rs` creates a Map containing collaborative Text.
It builds a Text Change from retain and insert operations.
It wraps that operation in a Map modify entry.
It applies the recursive Change to a concrete base Value.

```sh
cargo run -p colla --example basic_edit
```

## Binary round trip

`binary_roundtrip.rs` constructs a Value, calls `encode()`, then `decode()`.
It asserts structural equality between original and decoded values.

```sh
cargo run -p colla --example binary_roundtrip
```

## Concurrent edits

`collab_demo.rs` creates two Text Changes from one base.
It calls `transform_pair` with `TieBreak::LeftFirst`.
It applies each transformed path and asserts equal final Values.

```sh
cargo run -p colla --example collab_demo
```

Rust and JavaScript share the same Value model.
Rust constructors validate typed operation streams.
Rust errors are returned as `Result` values.
The examples use immutable Values.
They do not create Document sessions.
They do not define request identity.
They do not define authentication.
They do not define retry behavior.
They do not persist server history.
The collaboration example demonstrates pairwise TP1 only.
It does not prove TP2 or arbitrary peer-to-peer convergence.

## Adapting an example

Keep the base Value when implementing undo.
Use the application protocol for revision and ordering.
Use an envelope across a process boundary.
Add golden fixtures for cross-language bytes.
Add service tests for duplicate delivery.
Add service tests for reordered delivery.
Add input limits at untrusted boundaries.

## Next

Read [JavaScript Core](./javascript-core).
Read [OT concurrency](../ot/concurrency).
Read [production testing](../production/testing).
