# Authority

Authority is immutable server collaboration state for one document. It assigns revisions,
rebases submissions against retained history and records receipts for deduplication.
It does not write databases or authenticate callers.

## Accept a submission

`Authority.create({ documentId, value })` creates the initial state. `accept(submission)`
returns `{ authority, message }`; the original Authority is unchanged. Inspect
`message.type` to distinguish Commit from Rejection.

The returned Authority is the next state to adopt. Before connecting this operation to
a database or multiple request handlers, implement the [durable adoption sequence](/docs/production/persistence#server-durability).

## Snapshots and retained history

`snapshot()` produces a SyncSnapshot for new sessions. `commitsSince(revision)` supplies
retained Commits after that revision. If the requested basis has expired, it fails with
`history_expired`; see [Retries and revision gaps](./retries).

`compact(throughRevision)` returns a new Authority with an advanced rebase floor while
retaining deduplication receipts. Old clients may need [Recovery](./recovery). Choose
compaction retention with your application's offline requirements in mind.

## Restore server state

`Authority.restore(checkpoint)` restores the complete server state, including retained
history and receipts. A Value or SyncSnapshot alone cannot restore it. See
[Persistence](/docs/production/persistence) for storage and restart responsibilities.
