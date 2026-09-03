---
title: Values
description: Immutable structured values used by Colla Core.
---

# Values

<p class="lead">A Core Value is a complete, immutable document state. It can be a scalar, a sequence, or a recursively nested map and list.</p>

Values are the shared data model used by the Rust `colla` crate and the
JavaScript `colla-ot` package. The data model does not impose a document schema:
the root may be any Value, including a map or list.

## Value kinds

| Kind | JavaScript representation | Notes |
| --- | --- | --- |
| `null` | `null` | The null value. |
| `bool` | `boolean` | `true` or `false`. |
| `int` | `bigint` | Signed 64-bit integer; use `int()` to validate numbers and BigInts. |
| `float` | `number` | Finite IEEE-754 number; `NaN` and infinities are rejected. |
| `string` | `string` | Atomic UTF-8 text. It is replaced as a whole. |
| `text` | `{ type: "text", value: string }` | Collaborative text sequence. |
| `richtext` | `{ type: "richtext", spans: [...] }` | Attributed text and atomic embeds. |
| `list` | `readonly Value[]` | Ordered values. |
| `map` | `ValueMap` | String-keyed values. |

An ordinary JavaScript string is deliberately atomic. Use `text("...")` when
characters need independent retain, insert, and delete operations:

```ts
import { text, ValueHandle } from 'colla-ot'

const value = ValueHandle.fromJS({
  title: 'An atomic label',
  body: text('A collaboratively edited body'),
})
```

## JavaScript handles

`ValueHandle.fromJS(value, options?)` validates and stores a value in the Core
runtime. A handle can be inspected with `kind(path)`, `has(path)`, and
`get(path)`, converted back with `toJS()`, and serialized with `encode()`.
`clone()` creates an independent handle. Call `dispose()` when the handle is no
longer needed; operations on a disposed handle throw `CollaError` with code
`invalid_state`.

```ts
const document = ValueHandle.fromJS({ count: 1n, tags: ['ot', 'docs'] })

document.kind([])             // 'map'
document.kind(['tags'])       // 'list'
document.get(['count'])       // 1n
document.has(['missing'])     // false

const bytes = document.encode()
const restored = ValueHandle.decode(bytes)
```

Core operations never mutate an existing handle. Functions such as `apply()`
return a new handle, so retaining a prior value is safe for history and inverse
construction. See [Changes](/docs/core/changes) for the operations that
produce the next Value.

## Canonical values and limits

Construction freezes JavaScript-facing records and normalizes content. Map keys
and attribute keys use canonical UTF-8 ordering. `-0` becomes `0`; strings must
not contain unpaired UTF-16 surrogates. RichText spans are normalized as
described in [RichText](/docs/core/richtext).

`InputOptions.limits` can override individual safety limits, including maximum
depth, value nodes, container length, string bytes, sequence operations, and
sequence length. The defaults are intentionally finite; malformed or oversized
input raises a classified `CollaError` rather than creating a partial value.

For binary representation and strict decoding rules, see the
[protocol reference](/reference/protocol). For the complete JavaScript
surface, see the [JavaScript API reference](/reference/javascript).

Next: [Changes](/docs/core/changes).
