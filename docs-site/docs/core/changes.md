---
title: Changes
description: Canonical recursive operations over Colla Values.
---

# Changes

<p class="lead">A Change is a canonical forward operation relative to a known base Value. Applying it produces the next immutable Value.</p>

A Change contains no document ID, revision, author, timestamp, or global
operation ID. Those belong to the `Document` and synchronization layers. A
Change can be constructed independently, encoded, composed, inverted, and
transformed.

## Constructing a Change

The JavaScript API accepts a plain `ChangeInput` or a scoped builder:

```ts
import { Change } from 'colla-ot'

const change = Change.build(change => {
  change.map(map => {
    map.modify('title', title => {
      title.text(text => text.retain(5).insert(' v2'))
    })
  })
})
```

The root builder supports `noop()`, `replace(value)`, `map(...)`, `list(...)`,
`text(...)`, `richText(...)`, and checked integer addition with `intAdd(delta)`.
Map builders provide `insert`, `delete`, and recursive `modify`; list builders
provide `retain`, `insert`, `delete`, and `modify`. Sequence lengths are
Unicode-scalar lengths, not UTF-16 code-unit lengths.

## Operation kinds

| Change | Operations | Base requirement |
| --- | --- | --- |
| `noop` | Identity | Any Value. |
| `replace` | Replace the complete node | Any Value. |
| `int` | Add a signed 64-bit delta | The node must be an integer; overflow fails. |
| `map` | Insert, delete, or modify keys | The node must be a map. |
| `list` | Retain, insert, delete, or modify elements | The node must be a list. |
| `text` | Retain, insert, or delete scalars | The node must be Text. |
| `richtext` | Retain with attribute patch, insert, or delete | The node must be RichText. |

`Replace` may change a node's type. Primitive values support replacement only;
float addition is intentionally not provided. Applying a change checks type,
key presence, indexes, and sequence lengths against the supplied base.

## Canonical sequence rules

Constructors normalize operation streams so equivalent edits have stable
structure: zero-length retains and deletes disappear, compatible adjacent
operations merge, empty inserts disappear, and trailing plain retains are
omitted. A list `modify(noop)` becomes a retain. Map entries are unique and
sorted by key. These rules also apply to results of algebra operations.

An omitted sequence tail is an implicit retain. An insert followed by a delete
at the same position is ordered insert-first, which gives transformation a
deterministic input.

## Algebra

```ts
import { Change, apply, compose, invert, transformPair, ValueHandle } from 'colla-ot'

const before = ValueHandle.fromJS(1n)
const first = Change.fromJS({ type: 'int', delta: 2n })
const second = Change.fromJS({ type: 'int', delta: 3n })
const after = apply(before, first)
const undo = invert(first, before)
const roundTrip = apply(after, undo)

const [leftPrime, rightPrime] = transformPair(first, second, { order: 'left-first' })
const combined = compose(first, second)
```

`apply(base, change)` validates and returns a new handle. `compose(first,
second)` combines sequential changes. `invert(change, base)` needs the original
base because deletion and replacement data must be recovered from it.
`transformPair(left, right, { order })` returns changes that can be applied in
either order; choose `left-first` or `right-first` as the deterministic tie-break.

See the [OT model](/docs/ot/) for transform behavior and the
[protocol reference](/reference/protocol) for binary Change encoding.

Next: [Text](/docs/core/text).
