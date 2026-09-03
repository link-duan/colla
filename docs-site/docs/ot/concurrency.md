---
title: Concurrency and guarantees
description: TP1, deterministic tie-breaks, conflict behavior, and the TP2 boundary in Colla.
---

# Concurrency and guarantees

<p class="lead">Colla guarantees pairwise convergence for applicable concurrent Changes when every participant uses the same transform order.</p>

## TP1: pairwise convergence

Let `left` and `right` be Changes built from one base `v`. Transform returns
adjusted Changes and requires:

```text
(leftPrime, rightPrime) = transformPair(left, right, tieBreak)
apply(apply(v, left), rightPrime)
  ==
apply(apply(v, right), leftPrime)
```

This is TP1. It says that applying the two operations in either order reaches
the same Value. It does not mean that the operations are valid for every Value,
that a server accepted either operation, or that a three-way history is
path-independent.

## Deterministic tie-breaks

Content alone cannot order every conflict. Two inserts at one Text position,
two replacements, or two inserts for one Map key need a shared convention:

```rust
use colla::{transform_pair, TieBreak};

let (left_prime, right_prime) =
    transform_pair(&left, &right, TieBreak::LeftFirst)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`LeftFirst` gives the left argument precedence; `RightFirst` gives the right
argument precedence. The JavaScript `Document` rebase path always uses
`{ order: 'left-first' }`. A controller must preserve the same left/right
convention at every hop; replacing it with local timestamps changes the contract.

## Conflict behavior

| Concurrent Changes | Result |
| --- | --- |
| Replace vs child edit | Replace wins at that node; the child side becomes `Noop`. |
| Replace vs Replace | The tie-break selects the winning replacement. |
| Map delete vs modify | Delete wins; the modification is discarded. |
| Map modify vs modify | Child Changes are transformed recursively. |
| Map insert vs insert for one key | The tie-break selects one inserted value. |
| List delete vs element modify | Delete wins for the removed element. |
| List/Text inserts at one position | Both survive in the tie-break order. |
| Overlapping sequence deletes | The shared base range is deleted once. |
| RichText patches on different keys | Patches merge and both keys survive. |
| RichText patches on one key | The tie-break selects the resulting value. |
| RichText delete vs format | Delete wins for deleted content. |
| Integer add vs integer add | Both deltas remain checked additions; overflow fails. |

Changes with incompatible root kinds (for example, Text versus List) cannot
describe concurrent edits to one valid base and are rejected by transform.

## Rebase order matters

When several local Changes are pending, transform each one in queue order over
the current remote remainder. Keep the same confirmed revision and tie-break
through the complete transition, then commit state atomically. See
[Transform and rebasing](./transform-rebase) for the state equations.

## TP2 is outside the guarantee

TP2 asks whether a third concurrent operation reaches an equivalent transformed
form regardless of which earlier transformation path ran first. Colla does not
claim TP2. Core Changes intentionally omit stable operation identity, context
vectors, versions, and timestamps, so those facts cannot be reconstructed from
the Value/Change bytes.

Applications that need peer-to-peer history or a path-independent three-way
controller must add identity and context in their own protocol and choose an
algorithm whose assumptions they can enforce. Do not infer global convergence
from one successful TP1 example or from equal-looking encoded Changes.

## Controller checklist

- Establish one canonical base and server revision before transforming.
- Use one documented tie-break and left/right convention everywhere.
- Reject revision gaps, malformed Changes, and incompatible roots.
- Transform pending locals in FIFO order and commit atomically.
- Keep request identity, retries, deduplication, and history outside Core.
- Test TP1 properties for each Value kind and boundary error.

Next: [Protocol boundaries](./protocol-boundaries) and
[production testing](/docs/production/testing).

