<p align="center"><img src="docs/assets/logo-wordmark-dark.svg" alt="Colla" width="220" /></p>

# Colla

Immutable path-addressed structured documents, atomic editing, collaborative
undo/redo, and centralized synchronization.
Rust owns the semantic engine and codecs; the synchronous JavaScript
facade has zero npm runtime dependencies.

```ts
import { Document, History, text } from 'colla-ot'
const doc = Document.create({ tasks: [{ title: text('Draft') }, { title: text('Review') }] })
const history = History.attach(doc)
doc.edit(tx => {
  tx.text(['tasks', 0, 'title']).insert(5, ' updated')
  tx.list(['tasks']).move(0, 1)
})
console.log('Moved title:', doc.get(['tasks', 1, 'title'])?.toJS()) // Text: Draft updated
history.undo()
console.log('Restored title:', doc.get(['tasks', 0, 'title'])?.toJS()) // Text: Draft
history.close()
doc.close()
```

`colla` and `colla-ot` share development version **0.4.0**. No official release
has been published yet.

| Layer                       | Public objects                                                               |
| --------------------------- | ---------------------------------------------------------------------------- |
| Content and algebra         | Value, Change, apply, compose, invert, transform                             |
| Editing                     | Document, Transaction, scoped List/Text/RichText editors                     |
| Undo/redo                   | History, HistoryCheckpoint                                                   |
| Centralized synchronization | SyncSession, Authority, SyncSnapshot, Submission, ServerMessage, checkpoints |

Use `Value` for content snapshots, `SyncSnapshot` for confirmed server content
and `SessionCheckpoint` for complete offline recovery. Paths address content;
concurrent edits follow elements through insertions, deletions and List moves.
Mutable protocol payloads and Wasm handles are not part of the public API.

All algebra uses an explicit content base. The two-way `transform` result is
`[leftAfterRight, rightAfterLeft]` and satisfies TP1. TP2 and arbitrary peer-to-peer convergence are outside
the contract. Transport, databases, authentication, presence and editor adapters
belong to applications.

- [JavaScript API](https://link-duan.github.io/colla/reference/javascript)
- [Rust API](https://link-duan.github.io/colla/reference/rust)
- [Design and specifications](docs/README.md)
- [Changelog](CHANGELOG.md)

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
pnpm install --frozen-lockfile
pnpm check          # oxlint, oxfmt --check, typecheck, release scripts
pnpm format         # apply oxfmt
pnpm test:js
pnpm test:e2e
pnpm docs:build
```

`pnpm measure` records artifact sizes, editing/algebra/codec timings and memory.
Building the JS
package requires Rust's wasm32 target, wasm-pack, Node and pnpm. Publish is a
separate coordinated action described in the [release runbook](docs/internal/releasing.md).

## Documentation

[Read the Colla documentation](https://link-duan.github.io/colla/docs/getting-started/)
for Core, Editing, History, Sync, runnable examples and production integration.
