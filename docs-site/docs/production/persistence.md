---
title: Persistence and recovery
description: Persist Snapshots and pending Updates as an application-owned recovery record.
---

# Persistence, checkpoints, and recovery

`Document.snapshot()` captures the visible Core Value and its visible revision.
It intentionally omits pending Updates, listeners, transport connections, and
retry state. A Snapshot is therefore a content checkpoint, not a resumable
collaboration session.

## A recoverable record

If local delivery must survive a process crash, persist the Snapshot and the
outbound queue under one application-owned transaction or integrity boundary:

```text
checkpoint = {
  snapshotBytes,
  pendingUpdateBytes: [updateBytes...],
  protocolState,
}
```

| Field | Why it is needed | Owner |
| --- | --- | --- |
| `snapshotBytes` | Rebuild visible content and revision. | Colla envelope + storage |
| `pendingUpdateBytes` | Retry local edits that were not acknowledged. | Application queue |
| `protocolState` | Request IDs, cursor, auth/session metadata, or schema version. | Application protocol |

Do not serialize a JavaScript `Document` object or Wasm handle. Persist bytes
and plain application metadata, then construct a fresh `Document` during
recovery.

## Write ordering

1. Encode a new Snapshot from the current visible state.
2. Write the Snapshot and the queue state atomically, or use a journal that can
   prove which record is complete.
3. Remove an acknowledged queue entry only after the server acceptance is
   durable and the checkpoint reflects the corresponding revision.
4. Keep unacknowledged entries in FIFO order and retain their application
   request IDs.

An Update's `updateId` starts at `1n` for each new Document instance. It is a
local acknowledgement token, not a durable or globally unique deduplication
key. The application request ID must survive restoration if retries can repeat.

## Recovery sequence

```text
read record -> validate Snapshot -> create Document
            -> validate queue envelopes -> restore request metadata
            -> replay or resend in application-defined order
            -> apply only server-ordered remote revisions
```

Validate every envelope before exposing content to the editor. If a Snapshot
or queue entry is missing, truncated, or from an unsupported protocol version,
stop and request an application-level repair or fresh Snapshot. Never guess a
revision or apply an unrelated Update to bridge a gap.

## Format evolution and testing

Keep `COLLAS` and `COLLAU` envelope versions explicit. Reject trailing bytes
and unknown versions instead of silently accepting a future schema. Maintain
golden fixtures for Rust and JavaScript content, revisions, operation IDs, and
stable error codes. Test interrupted writes, partial records, duplicate queue
delivery, and recovery after a revision gap.

Next: [Synchronization protocol](./sync-protocol), then
[Production testing](./testing).
