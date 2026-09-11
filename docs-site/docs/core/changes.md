# Changes and OT algebra

A Change is an immutable ordered sequence of identity-targeted operations. It contains
an edit, not the base content or a server revision. All public algebra requires an
explicit Value base.

## Operations and construction

`Change.create(operations)` accepts insert, delete, set, move, text, add and richtext
operations. Structural targets and destination parents use ElementIds. Insert and Set
carry controlled Values. Operations execute sequentially against the working content.
`Change.noop()` creates an empty sequence; `isNoop`, `operations`, `encode` and `decode`
provide inspection and persistence. Use high-level editors for ordinary UI input.

## Algebra contracts

| Function | Result |
| --- | --- |
| `apply(base, change)` | New immutable Value, atomically |
| `invert(base, change)` | Change restoring original content and IDs |
| `compose(base, first, second)` | Equivalent to sequential application |
| `transform(base, left, right, { priority })` | `[leftAfterRight, rightAfterLeft]` |

For compose, second applies after first. For transform, left and right share the same
base. The returned pair satisfies TP1 for mergeable changes:
`apply(apply(base, left), rightAfterLeft)` equals
`apply(apply(base, right), leftAfterRight)`.

## Worked example

<<< ../../examples/core.ts

## Guarantees and boundaries

Priority resolves competing intent; it is not a timestamp or ID ordering. Structural
conflicts are explicit failures. TP2 and arbitrary peer-to-peer convergence are outside
the contract; use [SyncSession and Authority](/docs/sync/) for centralized synchronization.
