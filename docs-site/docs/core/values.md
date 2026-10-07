# Values

A Value is an immutable content tree. It carries no identity: equality is structural,
and two independently created Values with the same content are equal. A Value stays
readable and encodable after the Document it came from is closed.

## Value kinds

| JavaScript input  | Kind     | Supported edits                               |
| ----------------- | -------- | --------------------------------------------- |
| `null`            | Null     | Replace                                       |
| boolean           | Bool     | Replace                                       |
| bigint            | Int      | Replace; Add with checked signed i64 overflow |
| finite number     | Float    | Replace                                       |
| string            | String   | Replace as a whole                            |
| `text(string)`    | Text     | Insert and delete characters                  |
| `richText(spans)` | RichText | Insert and delete text and embeds; format     |
| array             | List     | Insert, delete and move items; edit inside    |
| plain object      | Map      | Insert, replace and delete keys; edit inside  |

Every kind can also be replaced by Set or removed from its parent by Delete.
`value.kind(path)` returns the lowercase kind name, such as `'text'` or `'map'`.

## Choose the right kind

- **String or Text.** A String is atomic: concurrent replacements resolve by
  [priority](./concurrency) and one of them wins. Use [Text](./text) for anything users
  type, so concurrent insertions in the same field merge.
- **Int or Float.** Use a bigint Int for counters: concurrent Add operations both
  apply. Float only supports replacement; NaN and infinities are rejected.
- **List or Map.** Use a List for ordered items that users insert, delete or reorder.
  Use a Map for named fields. List items are addressed by index, so concurrent edits
  are transformed as items shift.
- **Stable keys.** Values have no element identity. If your application needs a
  durable reference to an item, such as a database key or a selection anchor, store an
  `id` field in the item's Map.

## Create and inspect

```ts
import { Value, text } from 'colla-ot'

const value = Value.fromJS({
  title: text('Plan'),
  status: 'draft',
  votes: 3n,
  tags: ['urgent'],
})
console.log('Kind of title:', value.kind(['title'])) // text
console.log('Kind of status:', value.kind(['status'])) // string
console.log('Equal content:', value.equals(Value.fromJS(value.toJS()))) // true
```

`Value.fromJS` accepts the inputs in the table above, including existing Values.
Map input must be a plain object with data properties; symbol keys, accessors, class
instances and cyclic references are rejected.

## Projection is not persistence

`toJS()` returns a read-only projection in which Text and RichText appear as immutable
`Text` and `RichText` wrappers. The projection is convenient for rendering, but it is
not a storage format: JSON cannot represent bigint, and serializing would lose the
difference between String and Text. Persist with `encode()` and restore with
`Value.decode(bytes)`; the round trip is exact. The byte layout is specified in
[Protocol and encoding](/reference/protocol).

## Validity and limits

A Value is always valid. Construction, every edit and decoding enforce the same rules:

| Rule          | Limit                                              |
| ------------- | -------------------------------------------------- |
| Nesting depth | 100 levels                                         |
| Nodes         | 1,000,000 per Value, including RichText embeds     |
| String length | 16 MiB of UTF-8 per string, key or text            |
| Text          | Valid Unicode; unpaired UTF-16 surrogates rejected |
| Float         | Finite                                             |
| Int           | Signed 64-bit                                      |

Exceeding a size limit fails with `limit_exceeded`, a bigint outside i64 with
`integer_overflow`, a non-finite number or invalid Unicode with `invalid_value`, and an
unsupported input shape with `invalid_argument`. `Value.decode` rejects any bytes that would not
produce a valid Value with `invalid_encoding`, so decoded content needs no further
validation. See [Errors and resource limits](/docs/production/errors-limits).
