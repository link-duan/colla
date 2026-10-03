# colla

Immutable structured content, stable element identity, atomic editing, collaborative
undo/redo and centralized synchronization. The Rust crate owns state transitions and
canonical binary codecs. Requires Rust 1.81 or newer.

## Install

The first official release has not been published yet. Once 0.4 is available, add:

```toml
[dependencies]
colla = "0.4"
```

Element IDs draw their namespace from the operating system's secure entropy via
[`getrandom`](https://docs.rs/getrandom/0.2). On `wasm32-unknown-unknown`, select a
backend in your application, for example `getrandom = { version = "0.2", features = ["js"] }`
in a browser or Node host. The `colla-ot` npm package already does this.

## First edit

```rust
use colla::{Document, Value};

fn main() -> colla::Result<()> {
    let title = Value::text("Draft")?;
    let id = title.id();
    let doc = Document::create(title)?;
    doc.edit(|tx| tx.text_replace(id, 5, 0, " updated"))?;
    println!("Updated title: {:?}", doc.get(id)?.body()); // Text("Draft updated")
    doc.close()?;
    Ok(())
}
```

Rust text positions count Unicode scalars. The transaction callback returns a Result;
an error abandons its edits.

## Documentation

- [Getting started](https://link-duan.github.io/colla/docs/getting-started/)
- [Rust API](https://link-duan.github.io/colla/reference/rust)
- [Rust examples](https://link-duan.github.io/colla/docs/examples/rust)
- [Synchronization](https://link-duan.github.io/colla/docs/sync/)
