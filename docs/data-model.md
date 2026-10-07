# Content model

`Value` is an immutable owning tree of Null, Bool, Int, finite Float, atomic
String, collaborative Text, RichText, List and Map. Values carry no identity:
equality is structural, and two independently created Values with the same content
are equal. Text and RichText characters have positions. A RichText Embed is an
atomic Value with length one.

A Path is a sequence of Map keys and List indexes. It is the only way to address
content and is always interpreted against one content state: a snapshot, the
working content of a transaction, or the content produced by the preceding
operations of a Change. Paths are not stable across edits; collaborative
transformation rewrites the paths of concurrent operations instead.

| Edit                        | Effect                                          |
| --------------------------- | ----------------------------------------------- |
| Set Map member              | Write the member, inserting it when missing     |
| Set List element or root    | Replace the existing element                    |
| Insert                      | Add at a List position, shifting later elements |
| Delete                      | Remove an existing Map member or List element   |
| ListMove                    | Move one element within the same List           |
| Undo/redo and codec restore | Restore exact content                           |

Insert addresses only List positions; Map members are written with Set. Values are
immutable, so writing an existing Value elsewhere copies its content. Moving
content to another parent is a Delete followed by a write; concurrent
edits do not follow it. Cross-parent movement that preserves concurrent edits is
planned as an opt-in MovableTree type ([ADR 0008](adr/0008-path-addressed-core.md)).

Content limits are enforced by every construction, edit and decode: depth 100,
1,000,000 nodes including embeds, and 16 MiB of UTF-8 per string. `toJS()` is a
projection, not a codec.

See [JavaScript API](https://link-duan.github.io/colla/reference/javascript), [Rust API](https://link-duan.github.io/colla/reference/rust),
[glossary](../CONTEXT.md).
