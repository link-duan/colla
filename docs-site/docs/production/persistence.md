# Persistence and restart

Choose the durable object according to what must survive restart. Equal-looking content
alone is not enough to restore identities, undo intent or an unconfirmed request.

## Durable objects

| Object | Preserved state | Restore entry |
| --- | --- | --- |
| Value | Content and element IDs | Document.create(Value.decode(bytes)) |
| SyncSnapshot | Document ID, confirmed revision and Value | SyncSession.create |
| SessionCheckpoint | Confirmed basis, original request, rebased pending, buffer, local version, enabled History | SyncSession.restore |
| HistoryCheckpoint | Stacks, grouping, capacity and exact content basis | History.restore |
| AuthorityCheckpoint | History floor, retained commits and deduplication receipts | Authority.restore |

All objects use strict Rust-owned codecs. Store bytes without converting bigint fields
to JSON numbers. Value.toJS is a projection, not an identity-preserving durable format.

## Server durability

Serialize this sequence per document:

1. Read the current Authority and authorize the caller.
2. Call `accept(submission)` and inspect the returned message.
3. Durably store the returned Authority checkpoint.
4. Adopt that Authority as the current state.
5. Broadcast a Commit to all document clients, including its sender; return a Rejection only to its submitting client.

If storage fails, keep the previous Authority and do not announce the unpersisted result.
The client can retry after storage recovers. Await storage outside Document transactions.
Use a single document owner or storage concurrency control when multiple servers can
accept requests for the same document.

On restart, restore the checkpoint so request receipts survive. Persist compaction
before discarding older durable history.

## Client durability

Capture a SessionCheckpoint after relevant local and remote transitions when offline
work must survive a crash. The application chooses its write cadence and therefore its
crash-loss window. The following example resumes one writer from saved bytes:

<<< ../../examples/session-restart.ts

Store the bytes durably before stopping the old writer. Disconnect its transport handlers
as part of [runtime cleanup](/docs/editing/lifecycle#session-restart). For standalone
History, store its matching Value and HistoryCheckpoint together; see
[History restoration](/docs/history/checkpoints).

## Retention and recovery

Checkpoints can be larger than visible content: pending requests retain their original
basis, and History and Authority retain earlier changes. Colla does not impose a fixed
total byte-size limit on their envelopes. Choose storage, transport and retention limits
according to your application's capacity, and persist the complete checkpoint before
closing the old writer or announcing server acceptance.

Compaction can make old clients unable to rebase. Define a retention policy and implement
[explicit recovery](/docs/sync/recovery) before enabling aggressive log pruning.
The [sync example](/docs/examples/sync) shows where durable adoption belongs in a
message exchange. Add your database write at that boundary before broadcasting.
