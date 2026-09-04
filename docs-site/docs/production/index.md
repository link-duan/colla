---
title: Production integration
description: Define the application, server, persistence, and editor boundaries around Colla.
---

# Production integration

<p class="lead">Colla supplies deterministic content transitions and a small in-process Document state machine. Your system still defines delivery, identity, storage, and product policy.</p>

## The boundary map

| Layer | Owns | Does not own |
| --- | --- | --- |
| Core | Values, Changes, codecs, and OT algebra | Users, revisions, transport, or storage |
| `Document` | Visible/confirmed values, pending locals, rebase, and events | Network sessions, retries, or global deduplication |
| Application protocol | Document identity, requests, auth, retry, and recovery | Core Change semantics |
| Server | Durable ordering, accepted history, and checkpoints | Editor rendering and local Wasm handles |
| Editor adapter | UI transactions, selections, and render scheduling | Server revision assignment |

Keep these responsibilities separate so a protocol migration does not require
rewriting the editor reducer, and an editor migration does not change the wire
format.

## End-to-end flow

```text
editor input
    -> Document.transact(tx => ...)
    -> update.bytes (Uint8Array)
    -> application queue and transport
    -> server authentication, ordering, and persistence
    -> ordered Update bytes
    -> Document.applyRemote(bytes)
    -> immutable editSteps
    -> editor render
```

The visible value changes when `transact()` succeeds, before a server
acknowledgement. The server acknowledgement only advances confirmed state; it
does not create another visible edit. A remote Update must name the next
confirmed revision, or the application must recover through replay or a new
Snapshot.

## A minimal controller boundary

```ts
import { Document } from 'colla-ot'

// `queue`, `transport`, and `editor` are application-owned.
const document = Document.fromJS(initialValue, initialRevision)
const stop = document.subscribe(event => {
  if (event.origin === 'remote') editor.applyEditSteps(event.editSteps)
})

const update = document.transact(tx => {
  tx.set(['title'], 'New Title')
})
await queue.append({ requestId: crypto.randomUUID(), bytes: update.bytes })
const accepted = await transport.receiveOrdered()
document.applyRemote(accepted)
document.ack(update.updateId)
```

The sample shows ownership, not a required transport API. Add authorization,
request identity, retry state, and durable queue records in the controller.
Never use the instance-local `updateId` as a server-wide operation identity.

## Read the production guides

- [Synchronization protocol](./sync-protocol) defines ordering, retries, and
  revision-gap recovery.
- [Persistence and recovery](./persistence) defines checkpoint and queue
  durability without pretending a Snapshot is a session.
- [Security and limits](./security) separates structured-input policy from byte
  and request limits.
- [Errors and input limits](./errors-limits) gives a machine-readable failure
  handling pattern.
- [Testing and guarantees](./testing) turns these boundaries into assertions.

Next: [Synchronization protocol](./sync-protocol).
