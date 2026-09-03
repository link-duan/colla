# Install and make a first edit

<p class="lead">Install the package that matches your runtime, then verify the shared Core with one small edit.</p>

## JavaScript

```sh
pnpm add colla-ot
```

`colla-ot` is ESM-only and supports Node.js 22+, Vite 5+, and Rollup 4+.
Browser and Node entry points initialize the same WebAssembly core
synchronously; no public Wasm initialization function or bundler plugin is
required.

## Rust

```toml
[dependencies]
colla = "0.3"
```

The crate supports Rust 1.81 or newer. It is the reference implementation for
the data model, OT algebra, and canonical body codec.

## Verify JavaScript

```ts
import { Document, Change, text } from 'colla-ot'

const document = Document.fromJS(text('hello'))
const change = Change.build(change => {
  change.text(text => text.retain(5).insert(' world'))
})

document.applyLocal(change)
console.log(document.value().toJS())
// { type: 'text', value: 'hello world' }
```

If the value prints correctly, the package root is ready
to use. Continue to [Document synchronization](/docs/examples/javascript-document)
for a complete application flow, or see the [Rust examples](/docs/examples/rust).

## Verify Rust

```rust
use colla::{apply, Change, TextChange, TextOp, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let before = Value::text("hello");
    let change: Change = TextChange::from_ops([
        TextOp::Retain(5),
        TextOp::Insert(" world".into()),
    ])?.into();
    assert_eq!(apply(&before, &change)?, Value::text("hello world"));
    Ok(())
}
```

## Troubleshooting the first run

- Use `text()` when you need character-level editing. A normal string is atomic.
- Keep imports on the documented package exports; do not import generated
  Wasm files directly.
- If input is untrusted, configure structured-input limits and handle stable
  `CollaError` codes.
- Snapshot and Update bytes are versioned envelopes; raw Value and Change bytes
  are the corresponding body formats.
