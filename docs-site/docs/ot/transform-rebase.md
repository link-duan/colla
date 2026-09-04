---
title: Transform and rebasing
description: Rebase pending local Changes over ordered remote Updates with Colla's left-first rule.
---

# Transform and rebasing

<p class="lead">Pairwise transform adjusts two concurrent Changes from one base so either execution order reaches the same visible Value.</p>

## Transform a pair

Call `transformPair` with Changes that were independently made from the same
base Value. It returns `(leftPrime, rightPrime)`:

```ts
import { Change, transformPair } from 'colla-ot'

const [leftPrime, rightPrime] = transformPair(left, right, {
  order: 'left-first',
})
```

The pair satisfies TP1 whenever both transformed paths are applicable:

```text
apply(apply(base, left), rightPrime)
  ==
apply(apply(base, right), leftPrime)
```

The function does not assign a revision, identify an author, or send a message.
It only rewrites operation coordinates and conflict outcomes for this pair.

## Rebase a pending local Change

An optimistic Document has a confirmed Value and a visible Value that includes
pending local work. Suppose `local` was applied first and an ordered remote
Change arrives from the same confirmed revision:

```text
(localPrime, remotePrime) = transformPair(local, remote, left-first)
confirmedNext = apply(confirmedBase, remote)
visibleNext   = apply(apply(confirmedBase, local), remotePrime)
pendingNext   = localPrime
```

The `Document` performs this sequence for each pending local Change,
in FIFO order. It commits the confirmed and visible states together, then emits
one remote event for the resulting visible edit.

```ts
import { Document } from 'colla-ot'

const update = document.transact(tx => {
  tx.text(['title'], t => t.retain(5).insert(' v2'))
})
await transport.send(update.bytes)

document.applyRemote(await transport.receive())
```

The incoming Update must use exactly the current `confirmedRevision`. A gap or
future revision is not transformed speculatively; ask the application protocol
for replay or a fresh Snapshot.

## A concrete insertion

For a base Text `ab`, a local insert `X` and remote insert `Y` at the same
position are concurrent. With `left-first`, the local side keeps precedence and
the visible result is `aXYb` regardless of which transformed path is evaluated:

```text
base = ab
local  = retain(1), insert("X")
remote = retain(1), insert("Y")
visible after rebase = aXYb
```

Both inserted values survive. The tie-break only fixes their deterministic
order; it is not a conflict-resolution policy for unrelated application data.

## Failure atomicity

Remote processing can fail because of a revision mismatch, incompatible root
kinds, an invalid range, a missing Map key, or a transform/algebra error. The
Document rejects the complete transition and preserves the last committed
state. Do not apply a remote suffix, write placeholder content, or silently
retry against an unrelated base.

Acknowledgements are separate from rebasing. `ack(updateId)` advances the
confirmed state for the oldest pending local Update and emits no visible change
event. `updateId` is instance-local and must not be used as a global server
deduplication key.

## Keep the boundary explicit

- The server or application protocol orders remote Updates.
- The Document uses `left-first` for its local rebase path.
- The editor consumes immutable event `editSteps` after state commits.
- Transport, retries, request identity, history, and presence remain outside
  the OT operation.

Next: read [Concurrency](./concurrency), then the
[Document local/remote workflow](/docs/document/local-remote).

