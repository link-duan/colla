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

// `document` and `change` are application-owned values.
const snapshotBytes = document.snapshot().encode()
const snapshot = Snapshot.decode(snapshotBytes)
console.log(snapshot.revision, snapshot.content().toJS())

const update = document.applyLocal(change)
const decoded = Update.decode(update.encode())
console.log(decoded.revision, decoded.updateId, decoded.change())
```

## Strict decoding

Decoders reject wrong magic, unsupported versions, malformed or truncated
payloads, and trailing bytes. They raise `CollaError` with stable `code` and
`operation` fields; callers should reject the envelope rather than guessing a
schema or ignoring an extension. The [protocol reference](/reference/protocol)
contains the byte-level contract.

## Ownership

`Snapshot.content()` and `Update.change()` return independently owned handles.
`clone()` creates another owned envelope. Call `dispose()` when finished;
disposal is idempotent, while later methods throw `invalid_state`.

```ts
const snapshot = document.snapshot()
try {
  await storage.write('snapshot', snapshot.encode())
} finally {
  snapshot.dispose()
}
```

The envelopes carry no credentials, retry policy, request ID, global operation
identity, or transport metadata. Add those in an application-owned record.

Next: [Local and remote updates](/docs/document/local-remote).
