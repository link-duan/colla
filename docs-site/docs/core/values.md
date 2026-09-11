# Values and types

A Value is an immutable content tree. Each independently addressable node has a stable
ElementId, including the root and children of Maps and Lists. Reading or encoding a
Value does not depend on keeping its original Document open.

## JavaScript input

| Input | Colla kind | Editing behavior |
| --- | --- | --- |
| `null`, boolean | Null, Bool | Replace with Set |
| bigint | Int | Signed i64; checked increment |
| finite number | Float | Replace; no increment |
| string | String | Atomic replacement |
| `text(string)` | Text | Collaborative sequence |
| `richText(spans)` | RichText | Text, attributes and atomic embeds |
| `ref(id)` | Ref | Weak, one-hop reference |
| array | List | Ordered owning children |
| plain object | Map | String-keyed owning children |

`Value.fromJS(input)` constructs content; `value.get(pathOrId)` returns an immutable
subvalue or undefined when absent. `kind`, `has`, `idAt` and `pathOf` support inspection.
Objects must contain supported data properties: getters, symbols and cyclic ownership
are not an alternate input format. Reuse references through Ref instead of JS cycles.

## Projection and persistence

`toJS()` returns a content projection using immutable Text, RichText and Ref wrappers.
It omits owning IDs. Do not use JSON stringify/parse as a persistence codec: bigint is
not a JSON number and content projections cannot preserve element identities.
Use `Value.decode(value.encode())` for an identity-preserving round-trip.

## Equality and copying

`equals` compares content and owning identities. `contentEquals` ignores owning IDs
but still compares Ref target IDs literally. Two independently created equal-looking
trees can therefore be content-equal without being equal. `copy()` allocates fresh
IDs and remaps internal Refs. See [Move, Copy and Set](./move-copy-set).

[JavaScript Core example](/docs/core/changes) demonstrates immutable content and concurrent text operations.
