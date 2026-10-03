# colla

Immutable path-addressed structured content, atomic editing, collaborative
undo/redo and centralized synchronization. The Rust crate owns state transitions and
canonical binary codecs. Requires Rust 1.81 or newer.

## Install

The first official release has not been published yet. Once 0.4 is available, add:

```toml
[dependencies]
colla = "0.4"
```

## First edit

```rust
use colla::{Document, Segment, Value};

fn main() -> colla::Result<()> {
    let doc = Document::create(Value::map([("title".into(), Value::text("Draft")?)])?)?;
    let title = [Segment::Key("title".into())];
    doc.edit(|tx| tx.text_replace(&title, 5, 0, " updated"))?;
    println!("Updated title: {:?}", doc.get(&title)?.body()); // Text("Draft updated")
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
