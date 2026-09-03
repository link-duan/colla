---
title: Local and remote updates
description: Optimistic local edits, ordered remote Updates, rebasing, and acknowledgements.
---

# Local and remote updates

`applyLocal(change)` applies a Core Change to visible content immediately. It
returns an Update whose `revision` is the current visible revision before the
edit and whose `updateId` identifies the local queue entry. The application
owns sending and retrying its encoded bytes.

```ts
// `change` and `transport` are supplied by the application.
const update = document.applyLocal(change)
await transport.send(update.encode())
```

`applyRemote(update)` accepts only an Update whose revision equals the current
confirmed revision. It applies the remote Change to confirmed content, then
transforms each pending local Change against it using `{ order: 'left-first' }`.
The visible value is updated with the transformed remote Change and one remote
event is emitted.

```ts
const incoming = Update.decode(await transport.receive())
document.applyRemote(incoming)
```

## Acknowledgement

When the server accepts the oldest local operation, call:

```ts
document.ack(update.updateId)
update.dispose()
```

`ack()` advances confirmed state and rewrites revisions on remaining pending
Updates. It does not emit a change event. An unknown or out-of-order ID throws
`invalid_argument`; a restored Document has no knowledge of the old instance's
pending IDs.

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
