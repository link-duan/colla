# Identity-aware content

`Value` is an immutable owning tree of Null, Bool, Int, finite Float, atomic
String, collaborative Text, RichText, Ref, List and Map. Each independently
addressable root/Map member/List element has a stable ElementId. Text and
RichText characters have positions, not identities. RichText Embed is atomic.

IDs combine a random allocation namespace and positive monotonic sequence,
independent of client requests. Failed transactions do not rewind allocation.
Restore retains identities; a new runtime allocates later content in a fresh
namespace. Deterministic allocators support fixtures.

| Edit | Identity |
| --- | --- |
| Leaf edit/container member edit | Preserve edited element |
| Set existing | Preserve target root, import fresh descendants |
| Insert normal content | New subtree identities |
| Move | Preserve complete subtree |
| Copy | Fresh subtree identities, internal Ref remapping |
| Undo/redo | Restore original identities |
| Codec restore | Preserve all identities |

Refs are weak, same-document, atomic ID targets. They can dangle or form cycles.
One-hop resolution observes only the queried snapshot. Copy remaps targets
inside the copied range; outside targets stay unchanged. Set maps an imported
source root to the retained target root. Reverse references include Ref values
inside embeds, but embed content remains atomic to structural editing.

Full equality includes owner IDs. Content equality omits them but compares Ref
target IDs literally. `toJS()` is a projection, not an identity-preserving codec.
Owning identity duplication is invalid. Derived indexes are never encoded.

See [JavaScript API](https://link-duan.github.io/colla/reference/javascript), [Rust API](https://link-duan.github.io/colla/reference/rust),
[glossary](../CONTEXT.md) and [design contract](implementation-0.4.0.md).
