---
title: Snapshot and Update
description: Checkpoints and versioned changes in Document state.
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
await storage.write('doc.snapshot', snapshot.bytes)

const restored = Document.fromSnapshot(
  Snapshot.decode(await storage.read('doc.snapshot')),
)
```

`Snapshot.fromJS(value, revision?)` and `Snapshot.fromValue(value, revision?)`
are useful for adapters and tests. `snapshot.value` provides the direct JavaScript
Value tree; `snapshot.revision` is a `bigint`, and `snapshot.bytes` contains the
canonical `COLLAS` envelope.

## Update

An Update represents a versioned atomic change. In JavaScript, it is an immutable pure object:

| Field | Meaning |
| --- | --- |
| `revision` | Base revision when the change was created; an accepted remote Update must match the confirmed revision. |
| `updateId` | Instance-local token used for cumulative acknowledgement. |
| `bytes` | Canonical binary `COLLAU` envelope as `Uint8Array`. |

Use `document.transact()` for normal creation. `Update.decode(bytes)` validates
and unpacks the envelope without creating Wasm resource handles.

## Persistence boundary

A Snapshot can restore visible content and revision, but cannot resume an
outbound queue. If crash-safe delivery matters, persist queue records beside the
Snapshot and restore them in the application controller. Never infer pending
state from the Snapshot alone.

Next: [Envelope details](/docs/document/envelopes).
