---
title: Snapshot and Update envelopes
description: Binary layout, ownership, and strict decoding boundaries.
---

# Snapshot and Update envelopes

The local codec gives Snapshot and Update distinct, self-identifying
binary formats:

| Envelope | Header | Payload |
| --- | --- | --- |
| Snapshot | ASCII `COLLAS` | little-endian protocol `u16` version `1`, then a cocodec tuple `(revision, Value)` |
| Update | ASCII `COLLAU` | little-endian protocol `u16` version `1`, then a cocodec tuple `(revision, updateId, Change)` |

Both revision and ID fields are represented as unsigned 64-bit `bigint` values
in JavaScript. `Update.revision` identifies the Change base; it is not the
visible revision after local application.

```ts
import { Snapshot, Update } from 'colla-ot'

// `document` is an application-owned Document instance.
const snapshotBytes = document.snapshot().bytes
const snapshot = Snapshot.decode(snapshotBytes)
console.log(snapshot.revision, snapshot.value)

const update = document.transact(tx => tx.set(['title'], 'New Title'))
const decoded = Update.decode(update.bytes)
console.log(decoded.revision, decoded.updateId)
```

## Strict decoding

Decoders reject wrong magic, unsupported versions, malformed or truncated
payloads, and trailing bytes. They raise `CollaError` with stable `code` and
`operation` fields; callers should reject the envelope rather than guessing a
schema or ignoring an extension. The [protocol reference](/reference/protocol)
contains the byte-level contract.

## The zero-handle boundary

In JavaScript, `Snapshot` and `Update` are designed as pure, immutable data objects (POJOs).
Unlike lower-level Wasm handles (`ValueHandle`, `Change`, or `Document`), envelope objects:
- Hold no WebAssembly memory or native handles.
- Never require `dispose()` or `Symbol.dispose`.
- Rely entirely on standard JavaScript garbage collection.
- Expose canonical wire bytes directly via `.bytes: Uint8Array`.

```ts
const snapshot = document.snapshot()
// Write directly to disk or network with zero handle cleanup boilerplate:
await storage.write('snapshot', snapshot.bytes)
```

The envelopes carry no credentials, retry policy, request ID, global operation
identity, or transport metadata. Add those in an application-owned record.

Next: [Local and remote updates](/docs/document/local-remote).
