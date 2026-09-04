---
title: Local and remote updates
description: Optimistic local edits, ordered remote Updates, rebasing, and acknowledgements.
---

# Local and remote updates

`transact(fn)` applies mutations to visible content immediately and atomically.
It returns an immutable `Update` whose `revision` is the visible revision before the
edit, whose `updateId` identifies the local queue entry, and whose `bytes` contains
the pre-encoded canonical binary envelope ready for transport.

```ts
// `transport` is supplied by the application.
const update = document.transact(tx => {
  tx.set(['title'], 'New Title')
})
await transport.send(update.bytes)
```

`applyRemote(updateOrBytes)` accepts either a decoded `Update` or raw wire bytes
(`Uint8Array`) whose revision equals the current `confirmedRevision`. It applies
the remote Change to confirmed content, then transforms each pending local Change
against it using `{ order: 'left-first' }`. The visible value is updated with the
transformed remote Change and one remote event is emitted.

```ts
// Ingest incoming binary frames directly from network transport:
document.applyRemote(await transport.receive())
```

## Acknowledgement

When the server accepts a local operation, call:

```ts
document.ack(update.updateId)
```

`ack()` advances confirmed state cumulatively, clearing all pending local updates
up to and including `updateId`. It does not emit a change event because visible content
does not change. An unknown or out-of-order ID throws `invalid_argument`; a restored
Document has no knowledge of an old instance's pending IDs. `Update` objects are pure
JavaScript values and require no manual disposal.

## Ordering and gaps

```text
server revision r ── Update(revision r) ──> confirmed r+1
                                             │
                         pending local edits rebased here
```

Do not apply a skipped revision, duplicate envelope, or unrelated Update to
bridge a gap. The transport/controller should request replay from the last
confirmed revision or restore a trusted Snapshot. `updateId` is not a global
deduplication key and does not replace a server request ID.

All apply and transform work is failure-atomic: if validation, type checking,
revision checking, or rebasing fails, visible content, confirmed content, and
the pending queue remain unchanged.

Next: [Envelopes](/docs/document/envelopes).
