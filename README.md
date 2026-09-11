<p align="center"><img src="docs/assets/logo-wordmark-dark.svg" alt="Colla" width="220" /></p>

# Colla

Immutable structured documents with stable element identity, native Move and
Ref, atomic editing, collaborative undo/redo, and centralized synchronization.
Rust owns the semantic engine and version-2 codecs; the synchronous JavaScript
facade has zero npm runtime dependencies.

```ts
import { Document, History, text, ref } from "colla-ot"
const doc = Document.create({ tasks: [{ title: text("Draft") }], done: [], selected: null })
const history = History.attach(doc)
const task = doc.idAt(["tasks", 0])
doc.edit(tx => {
  tx.text(["tasks", 0, "title"]).insert(5, " v2")
  tx.set(["selected"], ref(task))
  tx.move(task, { parent: ["done"], index: 0 })
})
console.log("Path after move:", doc.pathOf(task)) // ["done", 0]
history.undo()
console.log("Path after undo:", doc.pathOf(task)) // ["tasks", 0]
history.close()
doc.close()
```

`colla` and `colla-ot` share development version **0.4.0**. No official release
has been published yet.

| Layer | Public objects |
| --- | --- |
| Content and algebra | Value, Change, ElementId, Ref, apply, compose, invert, transform |
| Editing | Document, Transaction, scoped List/Text/RichText editors |
| Undo/redo | History, HistoryCheckpoint |
| Centralized synchronization | SyncSession, Authority, SyncSnapshot, Submission, ServerMessage, checkpoints |

Use `Value` for content snapshots, `SyncSnapshot` for confirmed server content
and `SessionCheckpoint` for complete offline recovery. IDs survive moves,
undo/redo and codecs. Copies receive new identities and remap internal Refs.
Mutable protocol payloads and Wasm handles are not part of the public API.

All algebra uses an explicit content base. The two-way `transform` result is
`[leftAfterRight, rightAfterLeft]`. Mergeable operations satisfy TP1; structural
conflicts fail atomically. TP2 and arbitrary peer-to-peer convergence are outside
the contract. Transport, databases, authentication, presence and editor adapters
belong to applications.

- [JavaScript API](https://link-duan.github.io/colla/reference/javascript)
- [Rust API](https://link-duan.github.io/colla/reference/rust)
- [Design and specifications](docs/README.md)
- [Implementation contract](docs/implementation-0.4.0.md)
- [Changelog](CHANGELOG.md)

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
pnpm install --frozen-lockfile
pnpm check
pnpm test:js
pnpm test:e2e
pnpm docs:build
```

`pnpm measure` records artifact sizes, editing/algebra/codec timings and memory.
See [quality reports](docs/quality/) for before/after evidence. Building the JS
package requires Rust's wasm32 target, wasm-pack, Node and pnpm. Publish is a
separate coordinated action described in the [release runbook](docs/internal/releasing.md).

## Documentation

[Read the Colla 0.4 documentation](https://link-duan.github.io/colla/docs/getting-started/)
for Core, Editing, History, Sync, runnable examples and production integration.
