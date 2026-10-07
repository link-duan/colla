# Change algebra

Four functions operate on Changes. Each takes the base Value explicitly, because a
Change's Paths and positions only have meaning against the content they were made for.

## Contracts

| Function                                     | Inputs made against                       | Returns                                               |
| -------------------------------------------- | ----------------------------------------- | ----------------------------------------------------- |
| `apply(base, change)`                        | `base`                                    | The new Value                                         |
| `invert(base, change)`                       | `base`                                    | A Change that turns `apply(base, change)` into `base` |
| `compose(base, first, second)`               | `first` on `base`; `second` on its result | One Change equal to applying both in order            |
| `transform(base, left, right, { priority })` | Both on `base`                            | `[leftAfterRight, rightAfterLeft]`                    |

- **apply** is atomic. If any operation fails, it throws and no partial result exists.
  Int overflow is checked at every step, including intermediate steps of a composed
  Change.
- **invert** restores the exact base content. History stores inverses to implement undo.
- **compose** must equal sequential application:
  `apply(base, compose(base, a, b))` equals `apply(apply(base, a), b)`.
- **transform** takes two concurrent Changes and rewrites each to apply after the other.
  The result always satisfies TP1:

```
apply(apply(base, left), rightAfterLeft) == apply(apply(base, right), leftAfterRight)
```

All four functions throw rather than guessing when an input does not fit its base,
for example `out_of_bounds`, `missing_key` or `type_mismatch` from a Change made
against different content.

## Worked example

Two users edit `Hello` concurrently: one prepends `Say `, the other appends `!`.
Transform shifts the second edit past the prefix; compose and invert then combine and
undo the merged result.

<<< ../../examples/core.ts

## Priority

`priority` says which side wins when the two Changes express competing intent, such as
two insertions at the same position or two Sets of the same element. It is a stable
left/right rule, not a timestamp: callers choose it from their ordering model. Authority
gives already committed changes priority over rebased submissions, and History gives
remote changes priority over local undo. [Concurrent edits](./concurrency) lists every
case.

## Boundaries

TP1 is sufficient when one Authority orders all edits, which is how
[SyncSession and Authority](/docs/sync/) work. Colla does not promise TP2, so
transforming against several concurrent histories in different orders, as in
peer-to-peer merging, may diverge. Use the provided synchronization model rather than
building a peer-to-peer protocol on `transform`.
