---
title: OT guide
description: The Colla operational transformation model and its application boundaries.
---

# OT guide

<p class="lead">Colla OT turns immutable Values and recursive Changes into deterministic, testable state transitions.</p>

The algebra is intentionally smaller than a collaboration product. It operates
on one known base Value and leaves transport, sessions, identity, and editor
policy to the application around it.

## Choosing a workflow

Use `Document` when the edit must become a visible local transition and a
versioned `Update`. Use immutable values and operations directly when a reducer,
test, or protocol adapter owns the surrounding state.

## The four operations

| Operation | Meaning | Required context |
| --- | --- | --- |
| `apply(base, change)` | Evaluate one Change against a Value | The concrete base Value |
| `compose(first, second)` | Combine sequential Changes | `second` targets `apply(base, first)` |
| `invert(change, base)` | Build an undo Change | The original pre-change Value |
| `transformPair(left, right, tieBreak)` | Adjust concurrent Changes | Both Changes share one base and one ordering rule |

All four operations preserve immutable inputs. Invalid types, keys, ranges, or
checked arithmetic are reported as errors; they are not silently coerced into a
different operation.

## The algebraic contracts

For applicable Values and Changes, composition and inversion preserve these
identities:

```text
apply(apply(v, first), second) == apply(v, compose(first, second))
apply(apply(v, change), invert(change, v)) == v
```

For two concurrent Changes from `v`, transform provides pairwise convergence
(TP1):

```text
(leftPrime, rightPrime) = transformPair(left, right, tieBreak)
apply(apply(v, left), rightPrime)
  == apply(apply(v, right), leftPrime)
```

The equations are defined only when the operations are applicable. A Text
delete that consumes past the end, a Map insert for an existing key, or an
integer addition that overflows is a rejected transition.

## A typical pipeline

```ts
import { Change, ValueHandle, apply, text } from 'colla-ot'

const before = ValueHandle.fromJS(text('Draft'))
const edit = Change.build(change => {
  change.text(textEdit => textEdit.retain(5).insert(' v2'))
})
const after = apply(before, edit)
const bytes = edit.encode() // canonical Core Change body
```

Use the package-root `Document` when the edit must become a visible local
transition and a versioned `Update`. Use immutable values and operations directly
when a reducer, test, or protocol adapter only needs pure values and operations.

## What this guide covers

- [Apply, Compose, and Invert](./algebra) explains sequential semantics and why
  inversion needs the original Value.
- [Transform and rebasing](./transform-rebase) shows how pending local work is
  adjusted over an ordered remote Update.
- [Concurrency](./concurrency) documents TP1, tie-breaks, conflict behavior,
  and the explicit TP2 limitation.
- [Protocol boundaries](./protocol-boundaries) separates canonical bodies from
  Snapshot/Update envelopes and application-owned delivery policy.

Before choosing operations, review the [Core Changes model](/docs/core/changes)
and [Unicode coordinates](/docs/core/coordinates). For a complete state-machine
flow, continue to [Document local and remote updates](/docs/document/local-remote).

