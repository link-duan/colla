# Synchronization overview

Colla uses centralized Operational Transformation. Each document has an Authority that
orders submissions into Commits. Each client has a SyncSession with optimistic visible
content, a confirmed baseline and unconfirmed local work.

## End-to-end flow

1. The server creates a SyncSnapshot with `authority.snapshot()` and sends its `encode()` bytes.
2. The client decodes those bytes with `SyncSnapshot.decode` and creates a SyncSession with a unique active writer client ID.
3. Edits to `session.document` become visible immediately.
4. When `session.outbound()` returns a Submission, the client sends its `encode()` bytes.
5. The server decodes with `Submission.decode` and calls `authority.accept(submission)`.
6. The server follows the [durable adoption sequence](/docs/production/persistence#server-durability), then sends the returned message's encoded bytes. Commits go to every client, including the sender; Rejections go to the submitting client.
7. Clients decode with `ServerMessage.decode` and call `session.receive(message)`. Commits must arrive in revision order.

Snapshots, submissions and messages are controlled objects until encoded. The sender's
own Commit confirms its request; see [Submissions and commits](./submissions).

## Division of responsibility

Colla owns rebasing, message codecs, deduplication and checkpoints. Your application
owns sockets, scheduling, storage, authentication and authorization. A SyncSession is
client state, not a network connection.

## Reading path

Read [SyncSession](./session), [Submissions and commits](./submissions) and [Authority](./authority)
for each participant's role. The [two-client example](/docs/examples/sync) puts them
together in memory. Then add [Retries](./retries) and [Recovery](./recovery) to handle
delivery failures and unavailable history.
