---
title: Snapshot and Update
description: Checkpoints and versioned changes in the Document API.
---

# Snapshot and Update

`Snapshot` and `Update` are local binary envelopes around Core data. They are
not a network protocol: applications decide how to authenticate, route, retry,
and persist them.

## Snapshot

A Snapshot contains `(revision: u64, content: Value)`. It represents visible
content at one revision and intentionally excludes pending Updates,
acknowledgement state, listeners, transport connections, presence, and server
history.

```ts
import { Document, Snapshot } from 'colla-ot'

// `document` and `storage` are application-owned.
const snapshot = document.snapshot()
await storage.write('doc.snapshot', snapshot.encode())

const restored = Document.fromSnapshot(
  Snapshot.decode(await storage.read('doc.snapshot')),
)
```

`Snapshot.fromJS(value, revision?)` and `Snapshot.fromValue(value, revision?)`
are useful for adapters and tests. `snapshot.content()` returns an independently
owned `ValueHandle`; `snapshot.revision` is a `bigint`.

## Update

An Update contains `(revision: u64, updateId: u64, change: Change)`:

| Field | Meaning |
| --- | --- |
| `revision` | Visible revision when the Change was created; an accepted remote Update must match the confirmed revision. |
| `updateId` | Instance-local FIFO acknowledgement token. |
| `change()` | Independently owned Core Change. |

Use `document.applyLocal()` for normal creation. `Update.fromChange(revision,
updateId, change)` exists for adapters and tests. `Update.decode(bytes)` validates
the envelope before exposing its fields.

## Persistence boundary

A Snapshot can restore visible content and revision, but cannot resume an
outbound queue. If crash-safe delivery matters, persist queue records beside the
Snapshot and restore them in the application controller. Never infer pending
state from the Snapshot alone.

Next: [Envelope details](/docs/document/envelopes).
