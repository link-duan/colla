# Colla

Colla provides Rust-native Operational Transformation primitives for immutable,
nested values, plus a high-level Document model for JavaScript consumers. The
same data model, canonical binary format, and OT semantics are exposed through
the synchronous `colla-ot` package.

Colla includes:

- immutable `Value` trees and recursive, canonical `Change` values;
- Apply, Compose, Invert, and pairwise Transform with TP1 guarantees;
- collaborative Text and RichText with atomic embeds;
- typed Rust and JavaScript Change construction;
- a strict canonical binary body codec with configurable input limits.

The JavaScript package root also provides `Document`, `Snapshot`, and `Update`.
The low-level API remains available from `colla-ot/core`.

`Document` manages in-memory content and update state, but Colla is not a
complete collaborative document runtime. Applications still own sessions,
history, transport, presence, cursors, and editor-specific formats.

## APIs

- **High-level Document API** provides mutable document state, Snapshot
  persistence, local and remote Update handling, and change events.
- **Low-level Core API** provides immutable Values and Changes, canonical
  codecs, and Apply, Compose, Invert, and Transform. Use `colla-ot/core` from
  JavaScript or the `colla` crate from Rust.

## JavaScript

### High-level Document API

Use `Document` when the application needs to own current content, apply local
edits, receive ordered remote Updates, create persistent Snapshots, or update
an editor from change events.

```ts
import { Document } from "colla-ot"
import { Change, text } from "colla-ot/core"

const document = Document.fromJS(text("Draft"))
const unsubscribe = document.on("change", event => {
  console.log(event.origin, event.revision, event.editSteps)
})

const change = Change.build(change => {
  change.text(text => text.retain(5).insert(" v2"))
})

// The visible content changes immediately. Send this Update through your
// application-owned transport and acknowledge it after server acceptance.
const update = document.applyLocal(change)
const updateBytes = update.encode()

// Persist the current visible content and revision when a checkpoint is needed.
const snapshot = document.snapshot()
const snapshotBytes = snapshot.encode()

unsubscribe()
```

### Low-level Core API

Use `colla-ot/core` directly when immutable Value/Change operations are enough,
or when building custom state management on top of the OT primitives.

```ts
import { Change, ValueHandle, apply, text } from "colla-ot/core"

const before = ValueHandle.fromJS(text("Draft"))
const change = Change.build(change => {
  change.text(text => text.retain(5).insert(" v2"))
})
const after = apply(before, change)

console.log(after.toJS()) // { type: "text", value: "Draft v2" }
```

`colla-ot` supports Node.js 22+, Vite 5+, and Rollup 4+. See the
[JavaScript guide](packages/core/README.md) for Snapshot restoration, remote
Updates, acknowledgements, editor integration, codecs, and the complete Core
API.

## Rust Core API

```rust
use colla::{apply, Change, TextChange, TextOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let before = Value::text("Draft");
    let change: Change = TextChange::from_ops([
        TextOp::Retain(5),
        TextOp::Insert(" v2".into()),
    ])?
    .into();
    let after = apply(&before, &change)?;
    assert_eq!(after, Value::text("Draft v2"));
    Ok(())
}
```

The published `colla` crate supports Rust 1.81 or newer.

## Documentation

- [Official documentation site](https://link-duan.github.io/colla/)
- [Rust guide](crates/colla/README.md)
- [JavaScript guide](packages/core/README.md)
- [Rust API reference](https://docs.rs/colla)
- [Documentation index](docs/README.md)

The documentation index also links the normative specifications, architecture
decisions, roadmap, release runbook, domain language, and changelog.

Run `pnpm docs:dev` for a local preview of the official documentation site.

Run Rust tests with `cargo test --workspace --all-targets` and JavaScript
artifact tests with `pnpm test:js`.
