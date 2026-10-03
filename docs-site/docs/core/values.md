# Values and types

A Value is an immutable content tree addressed by [Paths](./paths). Values carry no
identity: two trees with the same content are equal. Reading or encoding a Value does
not depend on keeping its original Document open.

## JavaScript input

| Input             | Colla kind | Editing behavior                   |
| ----------------- | ---------- | ---------------------------------- |
| `null`, boolean   | Null, Bool | Replace with Set                   |
| bigint            | Int        | Signed i64; checked increment      |
| finite number     | Float      | Replace; no increment              |
| string            | String     | Atomic replacement                 |
| `text(string)`    | Text       | Collaborative sequence             |
| `richText(spans)` | RichText   | Text, attributes and atomic embeds |
| array             | List       | Ordered owning children            |
| plain object      | Map        | String-keyed owning children       |

`Value.fromJS(input)` constructs content; `value.get(path)` returns an immutable
subvalue or undefined when absent. `kind` and `has` support inspection. Objects must
contain supported data properties: getters, symbols and cyclic ownership are not an
alternate input format.

## Projection and persistence

`toJS()` returns a content projection using immutable Text and RichText wrappers. Do
not use JSON stringify/parse as a persistence codec: bigint is not a JSON number and
the projection does not distinguish String from Text. Use
`Value.decode(value.encode())` for an exact round-trip.

## Equality

`equals` compares content structurally. Values are immutable, so reuse one wherever
the same content is needed; see [Move, Copy and Set](./move-copy-set) for copying
inside a document.

[JavaScript Core example](/docs/core/changes) demonstrates immutable content and concurrent text operations.
