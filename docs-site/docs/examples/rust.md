# Rust

The Rust crate owns content, operations, editing, synchronization and canonical codecs.
These standalone programs use Rust Results and scalar text positions. After adding
`colla` to your Cargo dependencies, copy a program into `src/main.rs` and run `cargo run`.

## Edit, move and undo

<<< ../../../crates/colla/examples/basic_edit.rs

The edited item moves to the end of the List; undo restores its previous position and
content.

## Encode and restore

<<< ../../../crates/colla/examples/binary_roundtrip.rs

Value equality is structural. AuthorityCheckpoint restores server state, including the
history and request receipts that a content-only Value cannot preserve.

## Two clients

<<< ../../../crates/colla/examples/collab_demo.rs

Both clients converge to 3 after independent +1 and +2 edits. The example restores a
session through canonical checkpoint bytes. Its message loop is in memory; production
code must persist Authority state before broadcasting. Consult the [Rust reference](/reference/rust)
for the public method surface and the [Sync guide](/docs/sync/) for transport boundaries.
